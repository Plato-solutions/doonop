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

use crate::{searcher::Searcher, workload::Workload};
use engine::Engine;
use engine_factory::EngineFactory;
use log;
use log::{debug, info};
use serde_json::Value;
use shed::Sheduler;
use url::Url;

pub async fn crawl<S, F>(
    mut shed: S,
    mut engine_factory: F,
    urls: Vec<Url>,
    amount_searchers: usize,
) -> Vec<Value>
where
    S: 'static + Sheduler + Send,
    F: EngineFactory,
    <F::Backend as Searcher>::Error: Send + Sync,
    F::Backend: 'static + Send + Sync,
{
    // seed the pool
    //
    // it's important to seed engines before we start them.
    // In which case there might be a chance not to start it properly
    info!("seed {:?}", urls);
    shed.seed(urls).await;

    let mut engine_handlers = Vec::new();
    for _ in 0..amount_searchers {
        let engine = engine_factory.create().await.unwrap();
        let workload = shed.create_workload(engine);
        let handler = spawn_engine(workload);

        engine_handlers.push(handler);
    }

    info!("joining engine handlers");

    let mut data = Vec::new();
    for h in engine_handlers {
        let ext = h.await.unwrap();
        data.extend(ext);

        debug!("extend data");
    }

    data
}

fn spawn_engine<B, S>(mut w: Workload<B, S>) -> tokio::task::JoinHandle<Vec<Value>>
where
    B: 'static + Searcher + Send + Sync,
    B::Error: Send + Sync,
    S: 'static + Sheduler + Send,
{
    tokio::spawn(async move {
        let ext = w.start().await;
        ext
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::searcher::SearchResult;
    use crate::shed::{PoolSheduler, Sheduler};
    use async_trait::async_trait;
    use engine::tests::mock::MockBackend;
    use std::io;

    // it's expected that the test will make awake all engines
    #[tokio::test]
    #[should_panic(expected = "the test not written correctly; it's not concurent")]
    async fn crawl_test() {
        std::env::set_var("RUST_LOG", "debug");
        pretty_env_logger::init();

        let mut shed = PoolSheduler::default();

        let urls = vec![];

        // maybe it's worth to develop some move robust strategy then putting more the enough values
        let factory = MockFactory::new(
            vec![
                vec![Ok(SearchResult::new(vec![], "value1".into()))]
                    .into_iter()
                    .chain(std::iter::repeat(Ok(SearchResult::new(vec![], Value::Null))).take(5))
                    .collect(),
                vec![Ok(SearchResult::new(vec![], "value2".into()))]
                    .into_iter()
                    .chain(std::iter::repeat(Ok(SearchResult::new(vec![], Value::Null))).take(5))
                    .collect(),
                vec![Ok(SearchResult::new(vec![], "value3".into()))]
                    .into_iter()
                    .chain(std::iter::repeat(Ok(SearchResult::new(vec![], Value::Null))).take(5))
                    .collect(),
            ],
            shed.clone(),
        );

        let data = crawl(shed.clone(), factory, urls, 3);

        let urls = vec![
            Url::parse("https://example.net").unwrap(),
            Url::parse("https://wahoo.com").unwrap(),
            Url::parse("https://123.com").unwrap(),
        ];

        shed.seed(urls).await;

        let data = data.await;

        assert_eq!(
            data,
            vec![
                Value::from("value1"),
                Value::from("value2"),
                Value::from("value3")
            ]
        )
    }

    #[tokio::test]
    async fn crawl_test_single() {
        let shed = PoolSheduler::default();
        let urls = vec![Url::parse("https://example.net").unwrap()];
        let factory = MockFactory::new(
            vec![vec![
                Ok(SearchResult::new(
                    vec!["https://google.com".to_string()],
                    "value1".into(),
                )),
                Ok(SearchResult::new(vec![], "value2".into())),
            ]],
            shed.clone(),
        );

        let data = crawl(shed, factory, urls, 1).await;

        assert_eq!(data, vec![Value::from("value1"), Value::from("value2")])
    }

    struct MockFactory {
        results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>,
        sheduler: PoolSheduler,
    }

    impl MockFactory {
        pub fn new(
            results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>,
            sheduler: PoolSheduler,
        ) -> Self {
            MockFactory { results, sheduler }
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

            let mut engine = Engine::mock(results);
            use super::*;
            use crate::searcher::SearchResult;
            use async_trait::async_trait;
            use engine::tests::mock::MockBackend;
            use std::io;

            // it's expected that the test will make awake all engines
            #[tokio::test]
            #[should_panic(expected = "the test not written correctly; it's not concurent")]
            async fn crawl_test() {
                std::env::set_var("RUST_LOG", "debug");
                pretty_env_logger::init();

                let mut shed = PoolSheduler::default();

                let urls = vec![];

                // maybe it's worth to develop some move robust strategy then putting more the enough values
                let factory = MockFactory::new(
                    vec![
                        vec![Ok(SearchResult::new(vec![], "value1".into()))]
                            .into_iter()
                            .chain(
                                std::iter::repeat(Ok(SearchResult::new(vec![], Value::Null)))
                                    .take(5),
                            )
                            .collect(),
                        vec![Ok(SearchResult::new(vec![], "value2".into()))]
                            .into_iter()
                            .chain(
                                std::iter::repeat(Ok(SearchResult::new(vec![], Value::Null)))
                                    .take(5),
                            )
                            .collect(),
                        vec![Ok(SearchResult::new(vec![], "value3".into()))]
                            .into_iter()
                            .chain(
                                std::iter::repeat(Ok(SearchResult::new(vec![], Value::Null)))
                                    .take(5),
                            )
                            .collect(),
                    ],
                    shed.clone(),
                );

                let data = crawl(shed.clone(), factory, urls, 3);

                let urls = vec![
                    Url::parse("https://example.net").unwrap(),
                    Url::parse("https://wahoo.com").unwrap(),
                    Url::parse("https://123.com").unwrap(),
                ];

                shed.seed(urls).await;

                let data = data.await;

                assert_eq!(
                    data,
                    vec![
                        Value::from("value1"),
                        Value::from("value2"),
                        Value::from("value3")
                    ]
                )
            }

            #[tokio::test]
            async fn crawl_test_single() {
                let shed = PoolSheduler::default();
                let urls = vec![Url::parse("https://example.net").unwrap()];
                let factory = MockFactory::new(
                    vec![vec![
                        Ok(SearchResult::new(
                            vec!["https://google.com".to_string()],
                            "value1".into(),
                        )),
                        Ok(SearchResult::new(vec![], "value2".into())),
                    ]],
                    shed.clone(),
                );

                let data = crawl(shed, factory, urls, 1).await;

                assert_eq!(data, vec![Value::from("value1"), Value::from("value2")])
            }

            struct MockFactory {
                results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>,
                sheduler: PoolSheduler,
            }

            impl MockFactory {
                pub fn new(
                    results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>,
                    sheduler: PoolSheduler,
                ) -> Self {
                    MockFactory { results, sheduler }
                }
            }

            #[async_trait]
            impl EngineFactory for MockFactory {
                type Backend = MockBackend;

                async fn create(
                    &mut self,
                ) -> Result<Engine<Self::Backend>, <Self::Backend as Searcher>::Error>
                {
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

                    let mut engine = Engine::mock(results);

                    Ok(engine)
                }
            }

            Ok(engine)
        }
    }
}
