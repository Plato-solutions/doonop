// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    engine::{Engine, EngineId},
    engine_builder::EngineBuilder,
    engine_ring::EngineRing,
    searcher::Searcher,
};
use async_channel::{unbounded, Sender};
use log::{error, info};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::{io, sync::Arc};
use tokio::{sync::Notify, task::JoinHandle};
use url::Url;

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

/// Sheduler responsible for providing engines with *work*
///
/// Mainly the sheduler abstraction is developed in order to have an ability to identify that
/// To identifying that there's no more work.
/// We could check queeues but we could't guaranteee that some engine was doing work at the time.
/// And it's results could expand a state queues.
///
/// todo: do we need to develop a restore mechanism in case of engine error?
/// now not becouse engine is responsible for its errors but?
pub struct Workload<B, EB> {
    urls_pool: Vec<Url>,
    seen_list: HashSet<Url>,
    url_limit: Option<usize>,
    spawned_jobs: HashMap<EngineId, JoinHandle<()>>,
    ring: EngineRing<B, EB>,
}

impl<B, EB> Workload<B, EB>
where
    EB: EngineBuilder<Backend = B>,
    B: Searcher + Send + 'static,
{
    pub fn new(ring: EngineRing<B, EB>, url_limit: Option<usize>) -> Self {
        Self {
            url_limit,
            ring,
            urls_pool: Vec::new(),
            seen_list: HashSet::new(),
            spawned_jobs: HashMap::new(),
        }
    }

    pub async fn start(mut self, seed: Vec<Url>, notify: Arc<Notify>) -> Vec<Value> {
        let (sender, receiver) = unbounded();

        self.keep_urls(seed);
        self.spawn_engines(sender.clone()).await.unwrap();

        let mut results = Vec::new();
        let mut is_closed = false;
        loop {
            tokio::select! {
                Ok((engine, result)) = receiver.recv() => {
                    match result {
                        Ok((urls, data)) => {
                            results.push(data);
                            if self.inc_limit() {
                                is_closed = true;
                            }

                            self.keep_urls(urls);
                        }
                        Err(err) => {
                            error!("Engine {} got a error {}", engine.id, err);
                        }
                    }

                    self.return_engine(engine);

                    if !is_closed {
                        self.spawn_engines(sender.clone()).await.unwrap();
                    }

                    if self.spawned_jobs.len() == 0 {
                        break;
                    }
                }
                _ = notify.notified() => {
                    info!("Waiting for working engines");
                    is_closed = true;
                }
            }
        }

        results
    }

    fn filter_urls(&mut self, urls: Vec<Url>) -> Vec<Url> {
        let mut r = Vec::new();
        for url in urls.into_iter() {
            if self.seen_list.insert(url.clone()) {
                r.push(url)
            }
        }

        r
    }

    fn inc_limit(&mut self) -> bool {
        match self.url_limit.as_mut() {
            Some(0) => true,
            Some(limit) => {
                *limit -= 1;
                *limit == 0
            }
            None => false,
        }
    }

    fn keep_urls(&mut self, urls: Vec<Url>) {
        let urls = self.filter_urls(urls);
        self.urls_pool.extend(urls);
    }

    fn return_engine(&mut self, engine: Engine<B>) {
        info!("Return engine {}", engine.id);

        self.spawned_jobs.remove(&engine.id);
        self.ring.return_back(engine);
    }

    async fn spawn_engines(
        &mut self,
        sender: Sender<(Engine<B>, io::Result<(Vec<Url>, Value)>)>,
    ) -> io::Result<()> {
        loop {
            if self.spawned_jobs.len() >= self.ring.capacity() || self.urls_pool.is_empty() {
                break;
            }

            let url = self.urls_pool.pop().unwrap();
            let engine = self.ring.obtain().await?;
            let id = engine.id;

            info!("Spawn engine {} for url {}", id, url);

            let handler = spawn_engine(engine, url, sender.clone());

            // it's OK that it possibly rewrites an old handler which will drop it
            self.spawned_jobs.insert(id, handler);
        }

        Ok(())
    }
}

fn spawn_engine<B>(
    mut engine: Engine<B>,
    url: Url,
    sender: Sender<(Engine<B>, io::Result<(Vec<Url>, Value)>)>,
) -> JoinHandle<()>
where
    B: Searcher + Send + 'static,
{
    tokio::spawn(async move {
        let result = engine.run(url).await;
        sender.send((engine, result)).await.unwrap();
    })
}
