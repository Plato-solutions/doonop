use crate::engine::Engine;
use crate::filters::Filter;
use crate::searcher::{Searcher, WebDriverSearcher};
use crate::shed::Sheduler;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::Capabilities;
use thirtyfour::WebDriver;
use tokio::sync::Mutex;

#[async_trait]
pub trait EngineFactory {
    type Backend: Searcher;

    async fn create(&mut self)
        -> Result<Engine<Self::Backend>, <Self::Backend as Searcher>::Error>;
}

pub struct WebdriverFactory {
    id_counter: i32,
    state: Arc<Mutex<Sheduler>>,
    check_code: String,
    limit: Option<usize>,
    filters: Vec<Filter>,
    page_load_timeout: Duration,
}

impl WebdriverFactory {
    pub fn new(
        code: &str,
        limit: &Option<usize>,
        filters: &[Filter],
        page_load_timeout: Duration,
    ) -> Self {
        Self {
            id_counter: 0,
            state: Arc::default(),
            check_code: code.to_owned(),
            limit: *limit,
            filters: filters.to_vec(),
            page_load_timeout,
        }
    }

    pub fn create<S>(&mut self, searcher: S) -> Engine<S>
    where
        S: Searcher,
    {
        let engine = Engine {
            id: self.id_counter,
            shed: self.state.clone(),
            limit: self.limit,
            filters: self.filters.clone(),
            searcher,
        };

        self.id_counter += 1;

        engine
    }

    pub fn create_webdriver_engine(&mut self, wb: WebDriver) -> Engine<WebDriverSearcher> {
        self.create(WebDriverSearcher::new(wb, self.check_code.clone()))
    }

    pub fn sheduler(&self) -> Arc<Mutex<Sheduler>> {
        self.state.clone()
    }
}

#[async_trait]
impl EngineFactory for WebdriverFactory {
    type Backend = WebDriverSearcher;

    async fn create(
        &mut self,
    ) -> Result<Engine<Self::Backend>, <Self::Backend as Searcher>::Error> {
        let wb = create_webdriver(self.page_load_timeout).await;
        let engine = self.create_webdriver_engine(wb);
        Ok(engine)
    }
}

// todo: think about a way to have a support of webdrivers
// which doesn't backed by `xenon`.
//
// Where user don't only provides a number of jobs, but
// also a url connection for each job?
//
// todo: config of default URL
async fn create_webdriver(timeout: Duration) -> WebDriver {
    let mut cops = DesiredCapabilities::firefox();
    cops.set_headless().unwrap();

    // by this option we try to resolve CAPTCHAs
    cops.add("unhandledPromptBehavior", "accept").unwrap();

    let driver = WebDriver::new("http://localhost:4444", &cops)
        .await
        .unwrap();
    driver.set_page_load_timeout(timeout).await.unwrap();

    driver
}
