// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use async_trait::async_trait;
use serde_json::Value;
use snafu::{ResultExt, Snafu};
use thirtyfour::{error::WebDriverError, prelude::*};
use url::Url;

#[async_trait]
pub trait Searcher {
    async fn search(&mut self, url: &Url) -> Result<SearchResult, BackendError>;
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

#[derive(Debug, Snafu)]
pub enum BackendError {
    #[snafu(display("Unable to open an address {}: {}", address, source))]
    OpenAddress {
        source: WebDriverError,
        address: Url,
    },
    #[snafu(display("An error in running a script against {}: {}", address.as_str(), source))]
    RunningScript {
        source: WebDriverError,
        address: Url,
    },
    #[snafu(display("Unable to collect links on {}: {}", address, source))]
    CollectLinks {
        source: WebDriverError,
        address: Url,
    },
    #[snafu(display("{}", msg))]
    Other { msg: String },
}

impl BackendError {
    pub fn wb_error(&self) -> Option<&WebDriverError> {
        match &self {
            Self::RunningScript { source, .. } => Some(source),
            Self::OpenAddress { source, .. } => Some(source),
            Self::CollectLinks { source, .. } => Some(source),
            Self::Other { .. } => None,
        }
    }

    pub fn is_timeout(&self) -> bool {
        match self.wb_error() {
            Some(WebDriverError::Timeout(..)) => true,
            _ => false,
        }
    }

    pub fn address(&self) -> Option<&Url> {
        match &self {
            Self::RunningScript { address, .. } => Some(address),
            Self::OpenAddress { address, .. } => Some(address),
            Self::CollectLinks { address, .. } => Some(address),
            Self::Other { .. } => None,
        }
    }
}

pub struct WebDriverSearcher {
    driver: WebDriver,
    code: String,
}

#[async_trait]
impl Searcher for WebDriverSearcher {
    async fn search(&mut self, url: &Url) -> Result<SearchResult, BackendError> {
        self.driver.get(url.as_str()).await.context(OpenAddress {
            address: url.clone(),
        })?;

        let links = self
            .driver
            .find_elements(By::Tag("a"))
            .await
            .context(CollectLinks {
                address: url.clone(),
            })?;

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
                Err(err) => Err(err).context(CollectLinks {
                    address: url.clone(),
                })?,
            }
        }

        let data = self
            .driver
            .execute_script(&self.code)
            .await
            .context(RunningScript {
                address: url.clone(),
            })?
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
