// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use thirtyfour::prelude::*;
use url::Url;

#[async_trait]
pub trait Searcher {
    async fn search(&mut self, url: &Url) -> Result<SearchResult>;
    async fn close(self);
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub urls: Vec<String>,
    pub data: Value,
}

impl SearchResult {
    pub fn new(urls: Vec<String>, data: Value) -> Self {
        Self { urls, data }
    }
}

pub struct WebDriverSearcher {
    driver: WebDriver,
    code: String,
}

#[async_trait]
impl Searcher for WebDriverSearcher {
    async fn search(&mut self, url: &Url) -> Result<SearchResult> {
        self.driver
            .get(url.as_str())
            .await
            .context("Failed to open a url")?;
        let links = self
            .driver
            .find_elements(By::Tag("a"))
            .await
            .context("Failed to find links")?;

        let mut urls = Vec::new();
        for link in links {
            let href = link.get_attribute("href").await;
            match href {
                Ok(Some(href)) => {
                    urls.push(href);
                }
                Ok(None) | Err(thirtyfour::error::WebDriverError::StaleElementReference(..)) => {
                    continue
                }
                Err(err) => Err(err).context("Failed to get a link content")?,
            }
        }

        let data = self
            .driver
            .execute_script(&self.code)
            .await
            .context("Failed to execute a script")?
            .value()
            .clone();

        Ok(SearchResult { data, urls })
    }

    async fn close(self) {
        self.driver.quit().await.unwrap()
    }
}

impl WebDriverSearcher {
    pub fn new(driver: WebDriver, code: String) -> Self {
        Self { driver, code }
    }
}
