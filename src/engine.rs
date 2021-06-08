// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::filters::Filter;
use crate::searcher::Searcher;
use log::info;
use serde_json::Value;
use std::io;
use url::Url;

pub type EngineId = usize;

#[derive(Debug)]
pub struct Engine<B> {
    pub(crate) id: EngineId,
    pub(crate) filters: Vec<Filter>,
    pub(crate) backend: B,
}

impl<B: Searcher> Engine<B> {
    pub async fn run(&mut self, url: Url) -> io::Result<(Vec<Url>, Value)> {
        info!("engine {} working on {}", self.id, url);

        let result = self.backend.search(&url).await?;
        let found_urls = result.urls.len();
        let urls = self.filter_result(&result.urls, &url).await;

        info!(
            "engine {} found {} urls and filtered {}",
            self.id,
            found_urls,
            found_urls - urls.len()
        );

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
