// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::Clap;
use doonop::cfg::parse_cfg;
use doonop::{cfg::Cfg, crawl};
use log;
use log::info;
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() {
    turn_on_loggin();

    info!("Reading config");

    let cfg: Cfg = Cfg::parse();
    let crawl_config = parse_cfg(cfg).expect("Error occured while dealing with configuration file");

    info!("Config sucessfully read");

    let ctrl = Arc::new(Notify::new());
    spawn_ctrlc_handler(ctrl.clone());

    let (data, stats) = crawl(crawl_config, ctrl).await;

    info!("Praparing data for printing");
    info!(
        "Statistics: visited {}, collected {}, errors {}",
        stats.count_visited, stats.count_collected, stats.count_errors
    );

    for ext in data {
        println!("{}", ext);
    }
}

fn spawn_ctrlc_handler(ch: Arc<Notify>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("Received ctrl-c!");
        ch.notify_one();
        info!("Workload is notified!");
    })
}

fn turn_on_loggin() {
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
}
