// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod cfg;
pub mod engine;
pub mod engine_factory;
pub mod filters;
pub mod searcher;
pub mod shed;

use crate::engine_factory::EngineFactory;
use crate::searcher::Searcher;
use crate::searcher::WebDriverSearcher;
use engine::Engine;
use log;
use log::{debug, info};
use serde_json::Value;
use shed::Sheduler;
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::Capabilities;
use tokio::sync::Mutex;
use url::Url;

// pub struct Crawler<EF, S> {
//     engine_factory: EF,
//     sheduler: S,
//     amount_searchers: i32,
// }

// impl<EF, E, S> Crawler<EF, S>
// where
//     EF: EngineFactory<E>,
//     E: Engine,
//     S: EngineSheduler,
// {
//     pub async fn crawl() {
//         // run sheduler
//         // for responce from sheduler {
//         //
//         // }
//     }
// }

pub async fn crawl<S, F>(
    urls: Vec<Url>,
    state: Arc<Mutex<Sheduler>>,
    mut factory: F,
    amount_searchers: usize,
) -> Vec<Value>
where
    S: 'static + Searcher + Send + Sync,
    S::Error: Send + Sync,
    F: EngineFactory<Backend = S>,
{
    // seed the pool
    //
    // it's important to seed engines before we start them.
    // In which case there might be a chance not to start it properly
    for url in urls {
        info!("seed {}", url.as_str());
        state.lock().await.mark_url(url);
    }

    let mut engine_handlers = Vec::new();
    for _ in 0..amount_searchers {
        let engine = factory.create().await.unwrap();
        let handler = spawn_engine(engine);

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

fn spawn_engine<S>(mut engine: Engine<S>) -> tokio::task::JoinHandle<Vec<Value>>
where
    S: 'static + Searcher + Send + Sync,
    S::Error: Send + Sync,
{
    tokio::spawn(async move {
        let ext = engine.run().await;
        // let res = engine.close().await;
        // debug!("handler exit result {:?}", res);
        ext
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::searcher::SearchResult;
    use async_trait::async_trait;
    use engine::tests::mock::MockBackend;
    use std::io;

    // it's expected that the test will make awake all engines
    #[tokio::test]
    #[should_panic(expected = "the test not written correctly; it's not concurent")]
    async fn crawl_test() {
        panic!();

        std::env::set_var("RUST_LOG", "debug");
        pretty_env_logger::init();

        let shed: Arc<Mutex<Sheduler>> = Arc::default();

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

        let data = crawl(urls, shed.clone(), factory, 3);

        let urls = vec![
            Url::parse("https://example.net").unwrap(),
            Url::parse("https://wahoo.com").unwrap(),
            Url::parse("https://123.com").unwrap(),
        ];

        shed.lock().await.mark_urls(urls);

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
        let shed: Arc<Mutex<Sheduler>> = Arc::default();
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

        let data = crawl(urls, shed, factory, 1).await;

        assert_eq!(data, vec![Value::from("value1"), Value::from("value2")])
    }

    struct MockFactory {
        results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>,
        sheduler: Arc<Mutex<Sheduler>>,
    }

    impl MockFactory {
        pub fn new(
            results: Vec<Vec<Result<SearchResult, io::ErrorKind>>>,
            sheduler: Arc<Mutex<Sheduler>>,
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

            engine.shed = self.sheduler.clone();
            engine.id = self.results.len() as i32;

            Ok(engine)
        }
    }
}
