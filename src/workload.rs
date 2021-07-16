// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    backend::{Backend, BackendError},
    engine::{Engine, EngineId},
    engine_builder::EngineBuilder,
    engine_ring::EngineRing,
    retry::RetryPool,
};
use async_channel::{unbounded, Sender};
use log::{error, info};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    io,
    sync::Arc,
};
use tokio::{sync::Notify, task::JoinHandle};
use url::Url;

pub struct Workload<B, EB> {
    urls_pool: Vec<Url>,
    retry_policy: RetryPolicy,
    retry_pool: RetryPool,
    seen_list: HashSet<Url>,
    url_limit: Option<usize>,
    spawned_jobs: HashMap<EngineId, JoinHandle<()>>,
    ring: EngineRing<B, EB>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPolicy {
    RetryFirst,
    RetryLast,
    No,
}

#[derive(Debug, Default)]
pub struct Statistics {
    pub count_errors: usize,
    pub count_retries: usize,
    pub count_visited: usize,
    pub count_collected: usize,
}

impl<B, EB> Workload<B, EB>
where
    EB: EngineBuilder<Backend = B>,
    B: Backend + Send + 'static,
{
    pub fn new(
        ring: EngineRing<B, EB>,
        url_limit: Option<usize>,
        retry_policy: RetryPolicy,
        retry_pool: RetryPool,
    ) -> Self {
        Self {
            url_limit,
            ring,
            retry_policy,
            retry_pool,
            urls_pool: Vec::new(),
            seen_list: HashSet::new(),
            spawned_jobs: HashMap::new(),
        }
    }

    pub async fn start(mut self, seed: Vec<Url>, notify: Arc<Notify>) -> (Vec<Value>, Statistics) {
        if seed.is_empty() {
            return (Vec::new(), Statistics::default());
        }

        self.keep_urls(seed);
        let (sender, receiver) = unbounded();
        if let Err(err) = self.spawn_engines(sender.clone()).await {
            error!("Error occured while spawning engine {}", err);
            return (Vec::new(), Statistics::default());
        };

        let mut stats = Statistics::default();
        let mut results = Vec::new();
        let mut is_closed = false;
        loop {
            tokio::select! {
                Ok(EngineResult { engine, search_result }) = receiver.recv() => {
                    stats.count_visited += 1;

                    match search_result {
                        Ok((urls, data)) => {
                            results.push(data);
                            if self.inc_limit() {
                                is_closed = true;
                            }

                            self.keep_urls(urls);

                            stats.count_collected += 1;
                        }
                        Err(err) if err.is_timeout() && self.retry_policy != RetryPolicy::No => {
                            error!("Engine {} got a error {}; Put url back in the queue", engine.id, err);
                            stats.count_retries += 1;

                            let url = err.address().unwrap();
                            if !self.retry_pool.keep_retry(url.clone()) {
                                self.mark_visited(url.clone())
                            }
                        }
                        Err(err) => {
                            stats.count_errors += 1;
                            error!("Engine {} got a error {}", engine.id, err);
                        }
                    }

                    self.return_engine(engine);

                    if !is_closed {
                        // todo: unify a STOP interface
                        if let Err(err) = self.spawn_engines(sender.clone()).await {
                            error!("Error occured while spawning engine {}", err);
                            return (Vec::new(), Statistics::default());
                        };
                    }

                    if self.spawned_jobs.is_empty() {
                        break;
                    }
                }
                _ = notify.notified() => {
                    info!("Waiting for working engines");
                    is_closed = true;
                }
            }
        }

        (results, stats)
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

    fn mark_visited(&mut self, url: Url) {
        self.seen_list.insert(url);
    }

    fn get_url(&mut self) -> Option<Url> {
        match self.retry_policy {
            RetryPolicy::No => self.urls_pool.pop(),
            RetryPolicy::RetryFirst => self
                .retry_pool
                .get_url(self.urls_pool.is_empty())
                .or_else(|| self.urls_pool.pop()),
            RetryPolicy::RetryLast => self
                .urls_pool
                .pop()
                .or_else(|| self.retry_pool.get_url(self.urls_pool.is_empty())),
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

    async fn spawn_engines(&mut self, sender: Sender<EngineResult<B>>) -> io::Result<()> {
        while self.is_there_free_engine() {
            let url = self.get_url();
            if url.is_none() {
                break;
            };
            let url = url.unwrap();

            let engine = self.ring.obtain().await?;
            let id = engine.id;

            info!("Spawn engine {} for url {}", id, url);

            let handler = spawn_engine(engine, url, sender.clone());

            // it's OK that it possibly rewrites an old handler which will drop it
            self.spawned_jobs.insert(id, handler);
        }

        Ok(())
    }

    fn is_there_free_engine(&self) -> bool {
        self.ring.capacity() > self.spawned_jobs.len()
    }
}

struct EngineResult<B> {
    engine: Engine<B>,
    search_result: Result<(Vec<Url>, Value), BackendError>,
}

fn spawn_engine<B>(
    mut engine: Engine<B>,
    url: Url,
    sender: Sender<EngineResult<B>>,
) -> JoinHandle<()>
where
    B: Backend + Send + 'static,
{
    tokio::spawn(async move {
        let search_result = engine.run(url).await;
        sender
            .send(EngineResult {
                engine,
                search_result,
            })
            .await
            .unwrap();
    })
}
