// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// TODO: left issues
// proxy
// how to stop
// logging
// config
// ?engine manager?
// ...

// todo: think about concepts do we whan't to require check file?

// TODO: check && contribute?
// driver
//     .set_page_load_timeout(std::time::Duration::from_secs(15))
//     .await
//     .unwrap();

// todO: investigate proxy in thirtyfour

// TODO: validation of links by base_url|regex
//  provided by config

// TODO: ignore url by prefix or by regex?
// i.g. ignore all paths that start from /doc

// track engines to stop the process when all engines are returned (when all they shutdown)

// TODO:
// potentially we could collect a json provided by execution of a file
// it would allow us to make the tool more itself...

// todo: nice value to handle speed of engine?
// 1 - don't work, 0 - max effort(no sleep), -1 slow down | 1 spead up | 0 nornal
//
// too complex?

// MAIN ISSUES:
// captcha
// how we handle engine failure
// should it be restarted?
// should it put back a link?

// todo:
// A nice value for engines?
// We could have a variable provided by config
// in a range -1, 1
// -1 would mean don't have any explicit timeouts
// 1 would mean have a full timeout after a search
// 0 would mean calculate timeout as (cfg timeout - spend time) = timeout after serch
//
// the only question is do engines manage it on its own or we could manage it in another
// instance which would keep state and engines would go not directly in state
// but in this instance
//
// mainly this is an issue of abstactions
//
//
// todo: Move the crawl logic to a different module

use doonop::engine_builder::WebDriverConfig;
use doonop::{cfg::Cfg, crawl, filters::Filter};
use log;
use log::info;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use url::Url;

use clap::Clap;

#[tokio::main]
async fn main() {
    option_env!("RUST_LOG")
        .and_then(|_| {
            pretty_env_logger::init_timed();
            Some(())
        })
        .or_else(|| {
            pretty_env_logger::formatted_timed_builder()
                .filter_level(log::LevelFilter::Info)
                .init();
            Some(())
        });

    let cfg: Cfg = Cfg::parse();

    let page_load_timeout = cfg
        .page_load_timeout
        .map(|milis| Duration::from_millis(milis))
        .unwrap_or_else(|| Duration::from_secs(10));

    let amount_searchers = cfg.count_searchers.unwrap_or(1);

    let check = cfg.open_code_file().unwrap();
    let filters = cfg.filters().unwrap();

    let mut urls = Vec::new();
    cfg.urls_from_cfg(&mut urls).unwrap();
    cfg.urls_from_seed_file(&mut urls).unwrap();

    check_urs(&mut urls, &filters);

    let ctrl = Arc::new(Notify::new());

    spawn_ctrlc_handler(ctrl.clone());

    let data = crawl(
        WebDriverConfig {
            load_timeout: page_load_timeout,
        },
        check,
        filters,
        amount_searchers,
        cfg.limit,
        urls,
        ctrl,
    )
    .await;

    info!("prepare output");

    for ext in data {
        println!("{}", ext);
    }
}

fn spawn_ctrlc_handler(ch: Arc<Notify>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("ctrl-c received!");
        ch.notify_one();
        info!("workload is notified!");
    })
}

pub fn check_urs(urls: &mut Vec<Url>, filters: &[Filter]) {
    urls.sort();
    urls.dedup();
    urls.retain(|u| !filters.iter().any(|f| f.is_ignored(u)));
}
