use crate::filters::Filter;
use crate::shed::{Job, Sheduler};
use log::{debug, error, info, warn};
use serde_json::Value;
use std::sync::Arc;
use thirtyfour::prelude::*;
use tokio::sync::Mutex;
use tokio::time::delay_for;
use url::Url;

pub struct Engine<WebDriver> {
    pub(crate) id: i32,
    pub(crate) check: String,
    pub(crate) limit: Option<usize>,
    pub(crate) filters: Vec<Filter>,
    pub(crate) driver: WebDriver,
    pub(crate) shed: Arc<Mutex<Sheduler>>,
}

impl<WebDriver: WebDriverCommands + Sync> Engine<WebDriver> {
    pub async fn search(&mut self) -> Vec<Value> {
        debug!("start search on engine {}", self.id);

        let mut ext = Vec::new();
        loop {
            let mut guard = self.shed.lock().await;
            let job = guard.get_job(self.id);
            drop(guard);

            match job {
                Job::Search(url) => {
                    match self.examine_url(&mut ext, &url).await {
                        Ok(()) => (),
                        Err(err) => {
                            // don't put link back in the queue because it might be there forever
                            //
                            // todo: to get an ability put it back we have add a counter on a link how much times it was carried out.
                            // which is not as hard the only question do we whan't that ability?
                            match err {
                                thirtyfour::error::WebDriverError::Timeout(..) => {
                                    warn!("engine {}, timeout on processing link {}", self.id, url);
                                }
                                err => {
                                    error!("engine {} url {}, error {}", self.id, url, err);
                                }
                            }
                        }
                    };
                }
                Job::Idle(sleep) => {
                    info!("IDLE engine {}", self.id);
                    delay_for(sleep).await;
                }
                Job::Closed => {
                    info!("engine {} is about to be closed", self.id);
                    break;
                }
            }
        }

        debug!("stop search on engine {}", self.id);

        ext
    }

    async fn examine_url(&mut self, data: &mut Vec<Value>, url: &Url) -> WebDriverResult<()> {
        info!("engine {} processing {}", self.id, url);

        let (value, links) = self.search_url(url).await?;
        if !value.is_null() {
            data.push(value);

            if self.limit.map_or(false, |limit| data.len() > limit) {
                info!("engine {} has reached a limit", self.id);

                let mut guard = self.shed.lock().await;
                guard.close();
            }
        }

        if !links.is_empty() {
            let urls = validate_links(url, &links, &self.filters);

            let mut guard = self.shed.lock().await;
            guard.mark_urls(urls);
            drop(guard);
        }

        Ok(())
    }

    async fn search_url(&mut self, url: &Url) -> WebDriverResult<(Value, Vec<String>)> {
        self.driver.get(url.as_str()).await?;
        let links = self.driver.find_elements(By::Tag("a")).await?;

        let mut new_links = Vec::new();
        for link in links {
            let href = link.get_attribute("href").await;
            match href {
                Ok(href) => {
                    new_links.push(href);
                }
                Err(thirtyfour::error::WebDriverError::StaleElementReference(..)) => continue,
                Err(err) => return Err(err),
            }
        }

        let value = self
            .driver
            .execute_script(&self.check)
            .await?
            .value()
            .clone();

        Ok((value, new_links))
    }
}

impl Engine<WebDriver> {
    pub async fn shutdown(self) -> WebDriverResult<()> {
        debug!("engine {} shutdown", self.id);

        self.driver.quit().await
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

struct Proxy {
    // todo
// proxy-policy?
}

#[cfg(test)]
mod tests {
    use super::*;
    use thirtyfour::{SessionId, WebDriverSession};

    #[tokio::test]
    async fn engine_search() {
        let mut engine = Engine::mock(Vec::new(), Vec::new());
        let data = engine.search().await;
        assert!(data.is_empty())
    }

    #[tokio::test]
    async fn engine_with_data_search() {
        let mut engine = Engine::mock(
            Vec::new(),
            vec![
                Ok(Value::String("Hello Santa".into())),
                Ok(Value::Array(vec![10.into(), 20.into()])),
            ],
        );
        engine.shed.lock().await.mark_urls(vec![
            Url::parse("http://google.com").unwrap(),
            Url::parse("http://wahoo.com").unwrap(),
        ]);

        let data = engine.search().await;
        assert_eq!(
            data,
            vec![
                Value::Array(vec![10.into(), 20.into()]),
                Value::String("Hello Santa".into()),
            ]
        )
    }

    mod mock {
        use super::*;
        use thirtyfour::{common::command::Command, error::WebDriverResult};
        use tokio::sync::Mutex;

        impl Engine<MockWebDriver> {
            pub fn mock(open: Vec<WebDriverResult<()>>, exec: Vec<WebDriverResult<Value>>) -> Self {
                Engine {
                    id: 0,
                    check: String::new(),
                    limit: None,
                    filters: Vec::new(),
                    driver: MockWebDriver {
                        session: WebDriverSession::new(SessionId::null(), Arc::new(MockHttpClient)),
                        open_results: Mutex::new(open),
                        exec_results: Mutex::new(exec),
                    },
                    shed: Arc::default(),
                }
            }
        }

        #[derive(Debug)]
        pub struct MockWebDriver {
            session: WebDriverSession,
            open_results: Mutex<Vec<WebDriverResult<()>>>,
            exec_results: Mutex<Vec<WebDriverResult<Value>>>,
        }

        #[async_trait::async_trait]
        impl WebDriverCommands for MockWebDriver {
            fn session(&self) -> &WebDriverSession {
                &self.session
            }

            async fn get<S: Into<String> + Send>(&self, _: S) -> WebDriverResult<()> {
                let mut results = self.open_results.lock().await;
                let result = results.pop();

                match result {
                    Some(result) => result,
                    None => Ok(()),
                }
            }

            async fn execute_script<'a>(&'a self, _: &str) -> WebDriverResult<ScriptRet<'a>> {
                let mut results = self.exec_results.lock().await;
                let result = results.pop();

                match result {
                    Some(Ok(value)) => Ok(ScriptRet::new(&self.session, value.clone())),
                    Some(Err(err)) => Err(err),
                    None => Ok(ScriptRet::new(&self.session, Value::Null)),
                }
            }

            async fn find_elements<'a>(
                &'a self,
                _: By<'_>,
            ) -> WebDriverResult<Vec<WebElement<'a>>> {
                Ok(Vec::new())
            }
        }

        #[derive(Debug)]
        pub struct MockHttpClient;

        #[async_trait::async_trait]
        impl thirtyfour::http::connection_async::WebDriverHttpClientAsync for MockHttpClient {
            fn create(_: &str) -> WebDriverResult<Self>
            where
                Self: Sized,
            {
                Ok(Self)
            }

            async fn execute(
                &self,
                _: &SessionId,
                _: Command<'_>,
            ) -> WebDriverResult<serde_json::Value> {
                Ok(serde_json::Value::Null)
            }
        }
    }
}
