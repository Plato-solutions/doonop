// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
            check_code: code.to_owned(),
            limit: *limit,
            filters: filters.to_vec(),
            page_load_timeout,
        }
    }

    pub fn create_webdriver_engine(&mut self, wb: WebDriver) -> Engine<WebDriverSearcher> {
        self.create(WebDriverSearcher::new(wb, self.check_code.clone()))
    }

    pub fn create<S>(&mut self, backend: S) -> Engine<S>
    where
        S: Searcher,
    {
        let engine = Engine {
            filters: self.filters.clone(),
            backend,
        };

        engine
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
