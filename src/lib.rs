// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use engine_builder::{WebDriverConfig, WebDriverEngineBuilder};
use engine_ring::EngineRing;
use filters::Filter;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Notify;
use url::Url;
use workload::Workload;

pub mod cfg;
pub mod engine;
pub mod engine_builder;
pub mod engine_ring;
pub mod filters;
pub mod searcher;
pub mod workload;

#[derive(Debug)]
pub struct CrawlConfig {
    pub code: Code,
    pub wb_config: WebDriverConfig,
    pub filters: Vec<Filter>,
    pub count_engines: usize,
    pub url_limit: Option<usize>,
    pub urls: Vec<Url>,
}

#[derive(Debug)]
pub struct Code {
    pub text: String,
    pub code_type: CodeType,
}

#[derive(Debug)]
pub enum CodeType {
    Side,
    Js,
}

pub async fn crawl(config: CrawlConfig, ctrl: Arc<Notify>) -> Vec<Value> {
    let builder = WebDriverEngineBuilder::new(config.wb_config, config.code.text, config.filters);
    let ring = EngineRing::new(builder, config.count_engines);
    let workload = Workload::new(ring, config.url_limit);

    workload.start(config.urls, ctrl).await
}
