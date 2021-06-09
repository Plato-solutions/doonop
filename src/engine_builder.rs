// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{engine::Engine, filters::Filter, searcher::WebDriverSearcher};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::time::Duration;
use thirtyfour::{
    prelude::WebDriverResult, Capabilities, DesiredCapabilities, WebDriver, WebDriverCommands,
};

#[async_trait]
pub trait EngineBuilder {
    type Backend;

    async fn build(&mut self) -> Result<Engine<Self::Backend>>;
}

pub struct WebDriverEngineBuilder {
    config: WebDriverConfig,
    code: String,
    filters: Vec<Filter>,
    id: usize,
}

#[derive(Debug, Clone)]
pub struct WebDriverConfig {
    pub load_timeout: Duration,
}

impl WebDriverEngineBuilder {
    pub fn new(config: WebDriverConfig, code: String, filters: Vec<Filter>) -> Self {
        Self {
            config,
            code,
            filters,
            id: 0,
        }
    }
}

#[async_trait]
impl EngineBuilder for WebDriverEngineBuilder {
    type Backend = WebDriverSearcher;

    async fn build(&mut self) -> Result<Engine<Self::Backend>> {
        let wb = create_webdriver(&self.config)
            .await
            .context("Failed to create a webdriver")?;
        let searcher = WebDriverSearcher::new(wb, self.code.clone());
        let id = self.id;
        self.id += 1;
        let engine = Engine {
            backend: searcher,
            filters: self.filters.clone(),
            id,
        };

        Ok(engine)
    }
}

async fn create_webdriver(cfg: &WebDriverConfig) -> WebDriverResult<WebDriver> {
    let mut cops = DesiredCapabilities::firefox();
    cops.set_headless()?;

    // by this option we try to resolve CAPTCHAs
    cops.add("unhandledPromptBehavior", "accept")?;

    let driver = WebDriver::new("http://localhost:4444", &cops).await?;
    driver.set_page_load_timeout(cfg.load_timeout).await?;

    Ok(driver)
}
