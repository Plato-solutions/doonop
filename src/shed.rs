// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    engine::Engine,
    searcher::{SearchResult, Searcher},
    workload::Workload,
};
use async_channel::{Receiver, Sender};
use async_trait::async_trait;
use log::{info, warn};
use serde_json::Value;
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

/// Sheduler responsible for providing engines with *work*
///
/// Mainly the sheduler abstraction is developed in order to have an ability to identify that
/// To identifying that there's no more work.
/// We could check queeues but we could't guaranteee that some engine was doing work at the time.
/// And it's results could expand a state queues.
///
/// todo: do we need to develop a restore mechanism in case of engine error?
/// now not becouse engine is responsible for its errors but?
#[derive(Clone)]
pub struct Sheduler {
    seen_list: HashSet<Url>,
    engine_limit: Option<i32>,
    url_channel: Sender<Url>,
    result_channel: Receiver<(Vec<Url>, Value)>,
    processing_jobs: usize,
}

impl Sheduler {
    pub fn new(
        engine_limit: Option<i32>,
        url_channel: Sender<Url>,
        result_channel: Receiver<(Vec<Url>, Value)>,
    ) -> Self {
        Self {
            processing_jobs: 0,
            engine_limit,
            url_channel,
            result_channel,
            seen_list: Default::default(),
        }
    }

    pub async fn pool(&mut self) -> Vec<Value> {
        if self.processing_jobs == 0 {
            self.url_channel.close();
            return Vec::new();
        }

        let mut results = Vec::new();
        while let Ok((urls, data)) = self.result_channel.recv().await {
            results.push(data);
            if self.inc_limit() {
                break;
            }

            let urls = self.filter_urls(urls);
            self.seed_urls(urls).await;

            self.processing_jobs -= 1;
            if self.processing_jobs == 0 {
                break;
            }
        }

        info!("closing url channel");
        self.url_channel.close();

        results
    }

    pub async fn seed_urls(&mut self, urls: Vec<Url>) {
        self.processing_jobs += urls.len();

        for url in urls {
            self.url_channel.send(url).await.unwrap();
        }
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
        match self.engine_limit.as_mut() {
            Some(0) => true,
            Some(limit) => {
                *limit -= 1;
                *limit == 0
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod sheduler_tests {
    use super::Sheduler;
    use async_channel::unbounded;
    use std::time::Duration;
    use url::Url;

    #[tokio::test]
    async fn empty_sheduler_test() {
        let (result_s, result_r) = unbounded();
        let (url_s, url_r) = unbounded();

        let mut sheduler = Sheduler::new(None, url_s, result_r);
        let data = sheduler.pool().await;

        assert!(data.is_empty());
    }
}
