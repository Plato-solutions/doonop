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

use extension_search::{cfg::Cfg, engine::Engine, engine_factory::EngineFactory, shed::Sheduler};
use log;
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::Capabilities;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

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

    let check = cfg.open_code_file().unwrap();
    let filters = cfg.filters().unwrap();

    let mut urls = Vec::new();
    cfg.urls_from_cfg(&mut urls).unwrap();
    cfg.urls_from_seed_file(&mut urls).unwrap();

    urls.sort();
    urls.dedup();
    urls.retain(|u| !filters.iter().any(|f| f.is_ignored(u)));

    let mut mngr = EngineFactory::new(&check, &cfg.limit, &filters);

    // seed the pool
    //
    // it's important to seed engines before we start them.
    // In which case there might be a chance not to start it properly
    let state = mngr.sheduler();
    for url in urls {
        log::info!("seed {}", url.as_str());
        state.lock().await.mark_url(url);
    }

    let amount_searchers = cfg.count_searchers.unwrap_or(1);

    let mut engine_handlers = Vec::new();
    for _ in 0..amount_searchers {
        let driver = create_webdriver(cfg.page_load_timeout).await;
        let engine = mngr.create(driver);
        let handler = spawn_engine(engine);

        engine_handlers.push(handler);
    }

    spawn_ctrlc_handler(mngr.sheduler());

    log::info!("joining engine handlers");

    let mut data = Vec::new();
    for h in engine_handlers {
        let ext = h.await.unwrap();
        data.extend(ext);

        log::debug!("extend data");
    }

    log::info!("prepare output");

    for ext in data {
        println!("{}", ext);
    }
}

fn spawn_ctrlc_handler(state: Arc<Mutex<Sheduler>>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        log::info!("ctrl-c received!");
        state.lock().await.close();
        log::info!("engines notified about closing!");
    })
}

fn spawn_engine(mut engine: Engine) -> tokio::task::JoinHandle<Vec<String>> {
    tokio::spawn(async move {
        let ext = engine.search().await;
        let res = engine.shutdown().await;
        log::debug!("handler exit result {:?}", res);
        ext
    })
}

// todo: think about a way to have a support of webdrivers
// which doesn't backed by `xenon`.
//
// Where user don't only provides a number of jobs, but
// also a url connection for each job?
//
// todo: config of default URL
async fn create_webdriver(timeout: u64) -> WebDriver {
    let mut cops = DesiredCapabilities::firefox();
    cops.set_headless().unwrap();

    // by this option we try to resolve CAPTCHAs
    cops.add("unhandledPromptBehavior", "accept").unwrap();

    let driver = WebDriver::new("http://localhost:4444", &cops)
        .await
        .unwrap();
    driver
        .set_page_load_timeout(Duration::from_millis(timeout))
        .await
        .unwrap();

    driver
}
