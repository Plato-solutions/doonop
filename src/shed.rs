// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{engine::Engine, workload::Workload};
use async_trait::async_trait;
use std::{
    array::IntoIter,
    sync::{atomic::AtomicBool, Arc},
};
/// The module contains a shared state logic for engines.
/// How they get a link on check.
///
/// # Design
///
/// Sheduler is an internal state itself.
/// Why there wasn't used channels in order to remove Idle logic from engines?
/// Because it would mean that sheduler have to work concurently itself.
/// But what is more important that it would require implementing some logic how to balance engines.
/// Why? Becouse we have list of urls in wait_list which must be checked and we can't blindly split the list equally.
/// We also can't have a anlimited channels because by the same reason.
/// Limited channel would may block sometimes. Which denotes spliting state and sheduler.
///
/// Overall it might be not a bad idea but this is how things are done now.
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicI32,
};
use std::{sync::atomic::Ordering, time::Duration};
use tokio::sync::Mutex;
use url::Url;

/// Sheduler regulates what engines do
#[async_trait]
pub trait Sheduler: Sized + Sync + Send {
    /// pool returns a Job for an engine
    async fn pool(&mut self, engine: i32) -> Job;
    async fn seed(&mut self, urls: Vec<Url>);
    async fn stop(&mut self);
    fn create_workload<S>(&mut self, engine: Engine<S>) -> Workload<S, Self>;
}

/// Sheduler responsible for providing engines with *work*
///
/// Mainly the sheduler abstraction is developed in order to have an ability to identify that
/// To identifying that there's no more work.
/// We could check queeues but we could't guaranteee that some engine was doing work at the time.
/// And it's results could expand a state queues.
///
/// todo: do we need to develop a restore mechanism in case of engine error?
/// now not becouse engine is responsible for its errors but?
#[derive(Default, Clone)]
pub struct PoolSheduler {
    id_counter: Arc<AtomicI32>,
    engine_limit: Option<Arc<AtomicI32>>,
    engines: Arc<Mutex<HashMap<i32, EngineState>>>,
    seen_list: Arc<Mutex<HashSet<Url>>>,
    wait_list: Arc<Mutex<Vec<Url>>>,
    engines_stoped: Arc<AtomicBool>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Job {
    Search(Url),
    Idle(Duration),
    Closed,
}

// todo: might engine initiate a close?
#[derive(PartialEq, Eq)]
pub enum EngineState {
    Idle,
    // could hold a URL for recovery if there would be an error
    Work,
    Created,
}

#[async_trait]
impl Sheduler for PoolSheduler {
    async fn pool(&mut self, engine_id: i32) -> Job {
        // todo: does this method is too compex?
        // keeping a lock for too long is might a design smell

        if self.is_closed() {
            return Job::Closed;
        }

        if self
            .engines
            .lock()
            .await
            .iter()
            .all(|(_, s)| s == &EngineState::Idle)
            && self.wait_list.lock().await.is_empty()
        {
            self.stop().await;
            return Job::Closed;
        }

        let url = self.wait_list.lock().await.pop();
        match url {
            Some(url) => {
                self.set_engine_state(engine_id, EngineState::Work).await;
                self.dec_limit().await;
                Job::Search(url)
            }
            None => {
                self.set_engine_state(engine_id, EngineState::Idle).await;
                // todo: some logic with dynamic duration?
                Job::Idle(Duration::from_millis(5000))
            }
        }
    }

    async fn seed(&mut self, urls: Vec<Url>) {
        for url in urls {
            self.update_url(url).await
        }
    }

    async fn stop(&mut self) {
        self.engines_stoped
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn create_workload<S>(&mut self, engine: Engine<S>) -> Workload<S, Self> {
        let id = self
            .id_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Workload {
            engine,
            id,
            shed: self.clone(),
        }
    }
}

impl PoolSheduler {
    pub async fn dec_limit(&mut self) {
        match &self.engine_limit {
            Some(limit) => {
                if limit.load(Ordering::SeqCst) == 0 {
                    self.stop().await;
                } else {
                    limit.fetch_sub(1, Ordering::SeqCst);
                }
            }
            None => (),
        }
    }

    pub async fn mark_url(&mut self, url: Url) {
        self.update_url(url).await;
    }

    pub fn is_closed(&self) -> bool {
        self.engines_stoped
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) async fn set_engine_state(&mut self, id: i32, state: EngineState) {
        self.engines.lock().await.insert(id, state);
    }

    pub async fn update_url(&mut self, url: Url) {
        let mut seen_list = self.seen_list.lock().await;
        if !seen_list.contains(&url) {
            self.wait_list.lock().await.push(url.clone());
            seen_list.insert(url);
        }
    }
}

#[cfg(test)]
mod sheduler_tests {
    use crate::shed::PoolSheduler;

    use super::{Job, Sheduler};
    use std::time::Duration;
    use url::Url;

    #[tokio::test]
    async fn empty_sheduler_test() {
        let mut sheduler = PoolSheduler::default();
        let job = sheduler.pool(0).await;

        assert_eq!(job, Job::Closed);
    }

    #[tokio::test]
    async fn with_urls_test() {
        let urls = vec![
            Url::parse("http://locahost:8080").unwrap(),
            Url::parse("http://0.0.0.0:8080").unwrap(),
        ];

        let mut sheduler = PoolSheduler::default();
        sheduler.seed(urls.clone()).await;

        assert_eq!(sheduler.pool(0).await, Job::Search(urls[1].clone()));
        assert_eq!(sheduler.pool(0).await, Job::Search(urls[0].clone()));
        assert_eq!(sheduler.pool(0).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(0).await, Job::Closed);
    }

    #[tokio::test]
    async fn with_urls_with_multiple_engines_test() {
        let urls = vec![
            Url::parse("http://locahost:8080").unwrap(),
            Url::parse("http://0.0.0.0:8080").unwrap(),
        ];

        let mut sheduler = PoolSheduler::default();
        sheduler.seed(urls.clone()).await;

        assert_eq!(sheduler.pool(0).await, Job::Search(urls[1].clone()));
        assert_eq!(sheduler.pool(1).await, Job::Search(urls[0].clone()));
        assert_eq!(sheduler.pool(2).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(0).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(1).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(2).await, Job::Closed);
        assert_eq!(sheduler.pool(0).await, Job::Closed);
        assert_eq!(sheduler.pool(1).await, Job::Closed);
        assert_eq!(sheduler.pool(3).await, Job::Closed);
    }

    #[tokio::test]
    async fn with_urls_with_multiple_engines_dynamic_test() {
        let urls = vec![
            Url::parse("http://locahost:8080").unwrap(),
            Url::parse("http://0.0.0.0:8080").unwrap(),
        ];

        let mut sheduler = PoolSheduler::default();
        sheduler.seed(urls.clone()).await;

        assert_eq!(sheduler.pool(0).await, Job::Search(urls[1].clone()));
        assert_eq!(sheduler.pool(1).await, Job::Search(urls[0].clone()));
        assert_eq!(sheduler.pool(2).await, Job::Idle(Duration::from_secs(5)));

        let urls = vec![
            Url::parse("http://127.0.0.1:8080").unwrap(),
            Url::parse("http://8.8.8.8:60").unwrap(),
        ];
        sheduler.seed(urls.clone()).await;

        assert_eq!(sheduler.pool(2).await, Job::Search(urls[1].clone()));
        assert_eq!(sheduler.pool(1).await, Job::Search(urls[0].clone()));
        assert_eq!(sheduler.pool(0).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(1).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(2).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(2).await, Job::Closed);
        assert_eq!(sheduler.pool(0).await, Job::Closed);
        assert_eq!(sheduler.pool(1).await, Job::Closed);
        assert_eq!(sheduler.pool(3).await, Job::Closed);
    }

    #[tokio::test]
    async fn repeated_urls_test() {
        let urls = vec![
            Url::parse("http://locahost:8080").unwrap(),
            Url::parse("http://0.0.0.0:8080").unwrap(),
        ];

        let mut sheduler = PoolSheduler::default();
        sheduler.seed(urls.clone()).await;

        assert_eq!(sheduler.pool(0).await, Job::Search(urls[1].clone()));
        assert_eq!(sheduler.pool(1).await, Job::Search(urls[0].clone()));
        assert_eq!(sheduler.pool(2).await, Job::Idle(Duration::from_secs(5)));

        sheduler.seed(urls.clone()).await;

        assert_eq!(sheduler.pool(2).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(2).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(0).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(1).await, Job::Idle(Duration::from_secs(5)));
        assert_eq!(sheduler.pool(2).await, Job::Closed);
        assert_eq!(sheduler.pool(0).await, Job::Closed);
        assert_eq!(sheduler.pool(1).await, Job::Closed);
        assert_eq!(sheduler.pool(3).await, Job::Closed);
    }
}
