// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;
use thirtyfour::error::WebDriverError;
use thirtyfour::prelude::*;
use url::Url;

#[async_trait]
pub trait Searcher {
    type Error: Error;

    async fn search(&mut self, url: &Url) -> Result<SearchResult, Self::Error>;
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
    type Error = WebDriverError;

    async fn search(&mut self, url: &Url) -> Result<SearchResult, Self::Error> {
        self.driver.get(url.as_str()).await?;
        let links = self.driver.find_elements(By::Tag("a")).await?;

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
                Err(err) => return Err(err),
            }
        }

        let data = self
            .driver
            .execute_script(&self.code)
            .await?
            .value()
            .clone();

        Ok(SearchResult { data, urls })
    }
}

impl WebDriverSearcher {
    pub fn new(driver: WebDriver, code: String) -> Self {
        Self { driver, code }
    }

    pub async fn close(self) -> Result<(), <Self as Searcher>::Error> {
        self.driver.quit().await
    }
}
