// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use backend::Backend;
use engine_builder::{EngineBuilder, WebDriverConfig, WebDriverEngineBuilder};
use engine_ring::EngineRing;
use filters::Filter;
use retry::RetryPool;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;
use url::Url;
use workload::{RetryPolicy, Statistics, Workload};

pub mod backend;
pub mod cfg;
pub mod engine;
pub mod engine_builder;
pub mod engine_ring;
pub mod filters;
pub mod retry;
pub mod robots;
pub mod workload;

#[derive(Debug)]
pub struct CrawlConfig {
    pub code: Code,
    pub wb_config: WebDriverConfig,
    pub filters: Vec<Filter>,
    pub count_engines: usize,
    pub url_limit: Option<usize>,
    pub retry_policy: RetryPolicy,
    pub retry_threshold: Duration,
    pub retry_count: usize,
    pub robot_name: String,
    pub use_robots_txt: bool,
    pub urls: Vec<Url>,
}

#[derive(Debug)]
pub struct Code {
    pub text: String,
    pub code_type: CodeType,
}

#[derive(Debug)]
pub enum CodeType {
    Side,
    Js,
}

pub async fn crawl(config: CrawlConfig, ctrl: Arc<Notify>) -> (Vec<Value>, Statistics) {
    let builder = WebDriverEngineBuilder::new(
        config.wb_config.clone(),
        config.code.text.clone(),
        config.filters.clone(),
    );

    _crawl(config, builder, ctrl).await
}

async fn _crawl<B, Builder>(
    config: CrawlConfig,
    builder: Builder,
    ctrl: Arc<Notify>,
) -> (Vec<Value>, Statistics)
where
    Builder: EngineBuilder<Backend = B>,
    B: Backend + Send + 'static,
{
    let ring = EngineRing::new(builder, config.count_engines);
    let retry_pool = RetryPool::new(config.retry_threshold, config.retry_count);
    let workload = Workload::new(
        ring,
        config.url_limit,
        config.retry_policy,
        retry_pool,
        config.use_robots_txt,
        config.robot_name,
    );

    workload.start(config.urls, ctrl).await
}

#[cfg(test)]
mod tests {
    use std::{io, sync::Arc, time::Duration};

    use crate::{
        Code, CodeType, CrawlConfig, _crawl,
        backend::{Backend, BackendError, SearchResult},
        engine::Engine,
        engine_builder::{Browser, EngineBuilder, WebDriverConfig},
        workload::RetryPolicy,
    };
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use tokio::{sync::Notify, test};
    use url::Url;

    #[test]
    async fn crawl_with_single_engine() {
        let config = default_config(vec![Url::parse("http://example1.com").unwrap()], 1, None);
        let ctrl = Arc::new(Notify::new());
        let builder = MockBuilder::new(vec![MockBackend::new(vec![
            (
                &["http://example2.com", "http://example3.com"],
                json!("d1"),
                None,
            ),
            (&["http://example1.com"], json!("d2"), None),
            (&[], json!(null), None),
        ])]);

        let (data, _) = _crawl(config, builder, ctrl).await;

        assert_eq!(data, vec![json!("d1"), json!("d2"), json!(null)])
    }

    #[test]
    async fn crawl_with_2_engines() {
        let config = default_config(vec![Url::parse("http://example1.com").unwrap()], 2, None);
        let ctrl = Arc::new(Notify::new());
        let builder = MockBuilder::new(vec![
            MockBackend::new(vec![
                (
                    &["http://example2.com", "http://example3.com"],
                    json!("d1"),
                    None,
                ),
                (&[], json!("d2"), Some(Duration::from_millis(1000))),
            ]),
            MockBackend::new(vec![(&[], json!("d3"), None)]),
        ]);

        let (data, _) = _crawl(config, builder, ctrl).await;

        assert_eq!(data, vec![json!("d1"), json!("d3"), json!("d2")])
    }

    fn default_config(urls: Vec<Url>, count_engines: usize, limit: Option<usize>) -> CrawlConfig {
        CrawlConfig {
            wb_config: WebDriverConfig {
                load_timeout: Duration::from_secs(1),
                browser: Browser::Firefox,
                webdriver_address: Url::parse("http://localhost:4444").unwrap(),
                proxy: None,
            },
            robot_name: "DonoopRobot".to_string(),
            use_robots_txt: false,
            retry_policy: RetryPolicy::No,
            retry_count: 0,
            retry_threshold: Duration::from_secs(1),
            code: Code {
                text: String::new(),
                code_type: CodeType::Js,
            },
            filters: Vec::new(),
            url_limit: limit,
            urls,
            count_engines,
        }
    }

    struct MockBuilder {
        backends: Vec<MockBackend>,
        id: usize,
    }

    impl MockBuilder {
        fn new(backends: Vec<MockBackend>) -> Self {
            Self { backends, id: 0 }
        }
    }

    #[async_trait]
    impl EngineBuilder for MockBuilder {
        type Backend = MockBackend;

        async fn build(&mut self) -> io::Result<Engine<Self::Backend>> {
            if self.backends.is_empty() {
                panic!("Build call wasn't expected");
            }

            let backend = self.backends.remove(0);
            let id = self.id;
            self.id += 1;

            Ok(Engine::new(id, backend, &[]))
        }
    }

    struct MockBackend {
        results: Vec<(SearchResult, Option<Duration>)>,
    }

    impl MockBackend {
        fn new(results: Vec<(&[&str], Value, Option<Duration>)>) -> Self {
            let results = results
                .into_iter()
                .map(|(urls, data, timeout)| {
                    (
                        SearchResult::new(urls.iter().map(|url| url.to_string()).collect(), data),
                        timeout,
                    )
                })
                .collect();
            Self { results }
        }
    }

    #[async_trait]
    impl Backend for MockBackend {
        async fn search(&mut self, _: &Url) -> Result<SearchResult, BackendError> {
            if self.results.is_empty() {
                panic!("Search call wasn't expected");
            }

            let (result, sleep) = self.results.remove(0);
            if let Some(sleep) = sleep {
                tokio::time::sleep(sleep).await;
            }

            Ok(result)
        }

        async fn close(self) {}
    }
}
