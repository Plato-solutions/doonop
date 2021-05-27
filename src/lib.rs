// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod cfg;
pub mod engine;
pub mod engine_factory;
pub mod filters;
pub mod searcher;
pub mod shed;
pub mod workload;
pub mod workload_factory;

use crate::{searcher::Searcher, workload::Workload};
use async_channel::unbounded;
use engine::Engine;
use engine_factory::EngineFactory;
use log::{self, warn};
use log::{debug, info};
use serde_json::Value;
use shed::Sheduler;
use url::Url;
use workload_factory::WorkloadFactory;

pub async fn crawl<W, F>(
    mut sheduler: Sheduler,
    mut workload_factory: W,
    mut engine_factory: F,
    urls: Vec<Url>,
    amount_searchers: usize,
) -> Vec<Value>
where
    F: EngineFactory,
    <F::Backend as Searcher>::Error: Send + Sync,
    F::Backend: 'static + Send + Sync,
    W: WorkloadFactory,
{
    // start engines
    let mut engine_handlers = Vec::new();
    for _ in 0..amount_searchers {
        let engine = engine_factory.create().await.unwrap();
        let workload = workload_factory.create(engine).unwrap();
        let handler = spawn_engine(workload);

        engine_handlers.push(handler);
    }

    // seed the pool
    //
    // it's important to seed engines before we start them.
    // In which case there might be a chance not to start it properly
    info!("seed engines {:?}", urls);
    sheduler.seed_urls(urls).await;

    // start sheduler
    let data = sheduler.pool().await;

    info!("joining engine handlers");

    for h in engine_handlers {
        let id = h.await.unwrap();
        info!("engine {} is joined", id);
    }

    data
}

fn spawn_engine<B>(mut w: Workload<B>) -> tokio::task::JoinHandle<i32>
where
    B: 'static + Searcher + Send + Sync,
    B::Error: Send + Sync,
{
    tokio::spawn(async move {
        let id = w.id;
        w.start().await;
        id
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::searcher::SearchResult;
    use crate::shed::Sheduler;
    use async_trait::async_trait;
    use engine::tests::mock::MockBackend;
    use std::io;

    // it's expected that the test will make awake all engines
    #[tokio::test]
    async fn crawl_single_engine_test() {
        let (result_s, result_r) = unbounded();
        let (url_s, url_r) = unbounded();

        let shed = Sheduler::new(None, url_s.clone(), result_r.clone());
        let workload_factory = workload_factory::Factory::new(url_r.clone(), result_s.clone());

        // maybe it's worth to develop some move robust strategy then putting more the enough values
        let factory = MockFactory::new(vec![vec![
            Ok(SearchResult::new(vec![], "value1".into())),
            Ok(SearchResult::new(vec![], "value2".into())),
            Ok(SearchResult::new(vec![], "value3".into())),
        ]]);

        let urls = vec![
            Url::parse("https://123.dev").unwrap(),
            Url::parse("https://234.dev").unwrap(),
            Url::parse("https://356.dev").unwrap(),
        ];

        let data = crawl(shed.clone(), workload_factory, factory, urls, 1).await;

        assert_eq!(
            data,
            vec![
                Value::from("value1"),
                Value::from("value2"),
                Value::from("value3")
            ]
        )
    }

    // it's expected that the test will make awake all engines
    #[tokio::test]
    async fn crawl_multiply_engine_test() {
        let (result_s, result_r) = unbounded();
        let (url_s, url_r) = unbounded();

        let shed = Sheduler::new(None, url_s.clone(), result_r.clone());
        let workload_factory = workload_factory::Factory::new(url_r.clone(), result_s.clone());

        // we can't guarantee which engine will processs url
        // so we put all values to all engines
        let factory = MockFactory::new(vec![
            vec![
                Ok(SearchResult::new(vec![], "value1".into())),
                Ok(SearchResult::new(vec![], "value2".into())),
                Ok(SearchResult::new(vec![], "value3".into())),
            ],
            vec![
                Ok(SearchResult::new(vec![], "value1".into())),
                Ok(SearchResult::new(vec![], "value2".into())),
                Ok(SearchResult::new(vec![], "value3".into())),
            ],
            vec![
                Ok(SearchResult::new(vec![], "value1".into())),
                Ok(SearchResult::new(vec![], "value2".into())),
                Ok(SearchResult::new(vec![], "value3".into())),
            ],
        ]);

        let urls = vec![
            Url::parse("https://123.dev").unwrap(),
            Url::parse("https://234.dev").unwrap(),
            Url::parse("https://356.dev").unwrap(),
        ];

        let data = crawl(shed.clone(), workload_factory, factory, urls, 1).await;

        assert_eq!(
            data,
            vec![
                Value::from("value1"),
                Value::from("value2"),
                Value::from("value3")
            ]
        )
    }

    struct MockFactory {
        results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>,
    }

    impl MockFactory {
        pub fn new(results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>) -> Self {
            MockFactory { results }
        }
    }

    #[async_trait]
    impl EngineFactory for MockFactory {
        type Backend = MockBackend;

        async fn create(
            &mut self,
        ) -> Result<Engine<Self::Backend>, <Self::Backend as Searcher>::Error> {
            if self.results.is_empty() {
                panic!("unexpected call; check test")
            }

            let results = self
                .results
                .remove(0)
                .clone()
                .into_iter()
                .map(|r| r.map_err(|kind| io::Error::new(kind, "testing crawl erorr")))
                .collect();

            let engine = Engine::mock(results);

            Ok(engine)
        }
    }
}
