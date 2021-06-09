// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::filters::Filter;
use crate::searcher::Searcher;
use anyhow::Context;
use anyhow::Result;
use log::info;
use serde_json::Value;
use url::Url;

pub type EngineId = usize;

#[derive(Debug)]
pub struct Engine<B> {
    pub(crate) id: EngineId,
    pub(crate) filters: Vec<Filter>,
    pub(crate) backend: B,
}

impl<B: Searcher> Engine<B> {
    pub fn new(id: EngineId, backend: B, filters: &[Filter]) -> Self {
        Self {
            id,
            backend,
            filters: filters.to_vec(),
        }
    }

    pub async fn run(&mut self, url: Url) -> Result<(Vec<Url>, Value)> {
        info!("engine {} working on {}", self.id, url);

        let result = self
            .backend
            .search(&url)
            .await
            .context("Failed to run a page")?;
        let found_urls = result.urls.len();
        let urls = self.filter_result(&result.urls, &url);

        info!(
            "engine {} found {} urls and filtered {}",
            self.id,
            found_urls,
            found_urls - urls.len()
        );

        Ok((urls, result.data))
    }

    fn filter_result(&mut self, urls: &[String], url: &Url) -> Vec<Url> {
        validate_links(url, urls, &self.filters)
    }
}

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

#[cfg(test)]
mod tests {
    use super::validate_links;
    use url::Url;

    #[test]
    fn validate_link() {
        assert_eq!(
            validate_links(
                &Url::parse("https://example.net").unwrap(),
                &[
                    "https://example_1.net".into(),
                    "https://example_1.net?p1=123&p2=asd".into(),
                    "/path".into(),
                    "/path?p1=123&p2=asd".into()
                ],
                &[]
            ),
            vec![
                Url::parse("https://example_1.net").unwrap(),
                Url::parse("https://example_1.net?p1=123&p2=asd").unwrap(),
                Url::parse("https://example.net/path").unwrap(),
                Url::parse("https://example.net/path?p1=123&p2=asd").unwrap(),
            ]
        )
    }
}
