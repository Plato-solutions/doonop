// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use engine_builder::{WebDriverConfig, WebDriverEngineBuilder};
use engine_ring::EngineRing;
use filters::Filter;
use serde_json::Value;
use tokio::sync::Notify;
use url::Url;
use workload::Workload;

// keep a pool of Drivers not in Engine but in some other structure
// so we would spawn an engine on needs not keeping it as a task forewer.
// what it would solved?
// a issue that we don't know when to stop the engine
// down side is a subtle performace?????

// Kinda like it

// Remove _factory.rs as a outdated abstraction and see what will happen

// Use pantheon and .side files and optionally .JS files

pub mod cfg;
pub mod engine;
pub mod engine_builder;
pub mod engine_ring;
pub mod filters;
pub mod searcher;
pub mod workload;

pub async fn crawl(
    wb_cfg: WebDriverConfig,
    code: String,
    filters: Vec<Filter>,
    count_engines: usize,
    limit: Option<usize>,
    seed: Vec<Url>,
    ctrl: Arc<Notify>,
) -> Vec<Value> {
    let builder = WebDriverEngineBuilder::new(wb_cfg, code, filters);
    let ring = EngineRing::new(builder, count_engines);
    let workload = Workload::new(ring, limit);

    workload.start(seed, ctrl).await
}
