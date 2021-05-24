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

pub struct Engine<B> {
    pub(crate) filters: Vec<Filter>,
    pub(crate) backend: B,
}

impl<B: Searcher> Engine<B> {
    pub async fn run(&mut self, url: Url) -> Result<(Vec<Url>, Value), <B as Searcher>::Error> {
        let result = self.backend.search(&url).await?;
        let urls = self.filter_result(&result.urls, &url).await;
        Ok((urls, result.data))
    }

    async fn filter_result(&mut self, urls: &[String], url: &Url) -> Vec<Url> {
        validate_links(url, urls, &self.filters)
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

    pub mod mock {
        use super::*;
        use std::io;
        use tokio::sync::Mutex;

        impl Engine<MockBackend> {
            pub fn mock(results: Vec<Result<SearchResult, io::Error>>) -> Self {
                Engine {
                    filters: Vec::new(),
                    backend: MockBackend {
                        results: Mutex::new(results),
                    },
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
