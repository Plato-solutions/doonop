// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::filters::Filter;
use crate::searcher::{self, SearchResult, Searcher};
use crate::shed::{Job, Sheduler};
use log::{debug, error, info};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::sleep;
use url::Url;

// #[async_trait]
// pub trait Engine: Sync + Send {
//     async fn run<B: Backend>();
// }

// #[async_trait]
// pub trait EngineFactory {
//     type Backend: Searcher;

//     async fn create(&mut self)
//         -> Result<Engine<Self::Backend>, <Self::Backend as Searcher>::Error>;
// }

// #[async_trait]
// pub trait Sheduler {
//     async fn register<E: Engine>(&mut self, engine: E);
//     async fn is_closed(&self) -> bool;
// }

// #[async_trait]
// pub trait Workload {
//     async fn run<S: Sheduler, E: Engine>(
//         &mut self,
//         engine: E,
//         sheduler: S,
//     ) -> Result<E::Output, E::Error>;
// }

pub struct Engine<S> {
    pub(crate) id: i32,
    pub(crate) limit: Option<usize>,
    pub(crate) filters: Vec<Filter>,
    pub(crate) shed: Arc<Mutex<Sheduler>>,
    pub(crate) searcher: S,
}

impl<S: Searcher + Sync> Engine<S> {
    pub async fn run(&mut self) -> Vec<Value> {
        debug!("start search on engine {}", self.id);

        let mut data = Vec::new();
        loop {
            let mut guard = self.shed.lock().await;
            let job = guard.get_job(self.id);
            drop(guard);

            match job {
                Job::Search(url) => {
                    info!("engine {} processing {}", self.id, url);

                    match self.searcher.search(&url).await {
                        Ok(result) => self.save_result(result, &url, &mut data).await,
                        Err(err) => {
                            // don't put link back in the queue because it might be there forever
                            //
                            // todo: to get an ability put it back we have add a counter on a link how much times it was carried out.
                            // which is not as hard the only question do we whan't that ability?
                            error!("engine {} url {}, error {}", self.id, url, err);
                        }
                    };
                }
                Job::Idle(duration) => {
                    info!("IDLE engine {} for {:?}", self.id, duration);
                    sleep(duration).await;
                }
                Job::Closed => {
                    info!("engine {} is about to be closed", self.id);
                    break;
                }
            }
        }

        debug!("stop search on engine {}", self.id);

        data
    }

    async fn save_result(&mut self, result: SearchResult, url: &Url, data: &mut Vec<Value>) {
        if !result.urls.is_empty() {
            let urls = validate_links(url, &result.urls, &self.filters);

            let mut guard = self.shed.lock().await;
            guard.mark_urls(urls);
            drop(guard);
        }

        // do we need check on `null` here?
        data.push(result.data);

        if self.limit.map_or(false, |limit| data.len() > limit) {
            info!("engine {} has reached a limit", self.id);

            let mut guard = self.shed.lock().await;
            guard.close();
        }
    }
}

impl Engine<searcher::WebDriverSearcher> {
    pub async fn close(self) -> Result<(), <searcher::WebDriverSearcher as Searcher>::Error> {
        self.searcher.close().await
    }
}

//todo: logging?
fn validate_links(base: &Url, links: &[String], filters: &[Filter]) -> Vec<Url> {
    links
        .iter()
        .filter_map(|link| make_absolute_url(base, &link))
        .filter(|l| !filters.iter().any(|f| f.is_ignored(l)))
        .map(|mut l| {
            // remove fragments to reduce count of dublicates
            // we do it after filters cause someone could match agaist fragments
            // but from address point of view they are meanless
            l.set_fragment(None);
            l
        })
        .collect()
}

fn make_absolute_url(base: &Url, url: &str) -> Option<Url> {
    match Url::parse(url) {
        Ok(url) => Some(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => match base.join(&url) {
            Ok(url) => Some(url),
            Err(..) => None,
        },
        Err(..) => None,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn engine_search() {
        let mut engine = Engine::mock(Vec::new());
        let data = engine.run().await;
        assert!(data.is_empty())
    }

    #[tokio::test]
    async fn engine_with_data_search() {
        let mut engine = Engine::mock(vec![
            Ok(SearchResult::new(
                Vec::new(),
                Value::String("Hello Santa".into()),
            )),
            Ok(SearchResult::new(
                Vec::new(),
                Value::Array(vec![10.into(), 20.into()]),
            )),
        ]);
        engine.shed.lock().await.mark_urls(vec![
            Url::parse("http://google.com").unwrap(),
            Url::parse("http://wahoo.com").unwrap(),
        ]);

        let data = engine.run().await;
        assert_eq!(
            data,
            vec![
                Value::String("Hello Santa".into()),
                Value::Array(vec![10.into(), 20.into()]),
            ]
        )
    }

    pub mod mock {
        use super::*;
        use std::io;
        use tokio::sync::Mutex;

        impl Engine<MockBackend> {
            pub fn mock(results: Vec<Result<SearchResult, io::Error>>) -> Self {
                Engine {
                    id: 0,
                    limit: None,
                    filters: Vec::new(),
                    searcher: MockBackend {
                        results: Mutex::new(results),
                    },
                    shed: Arc::default(),
                }
            }
        }

        #[derive(Debug)]
        pub struct MockBackend {
            results: Mutex<Vec<Result<SearchResult, io::Error>>>,
        }

        #[async_trait::async_trait]
        impl Searcher for MockBackend {
            type Error = io::Error;

            async fn search(&mut self, _: &Url) -> Result<SearchResult, Self::Error> {
                if self.results.lock().await.is_empty() {
                    panic!("unexpected call of search; please check test");
                }

                self.results.lock().await.remove(0)
            }
        }
    }
}
