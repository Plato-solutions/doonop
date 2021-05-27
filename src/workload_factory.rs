// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::Engine;
use crate::filters::Filter;
use crate::searcher::{Searcher, WebDriverSearcher};
use crate::shed::Sheduler;
use crate::workload::Workload;
use async_channel::Receiver;
use async_channel::Sender;
use async_trait::async_trait;
use serde_json::Value;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::Capabilities;
use thirtyfour::WebDriver;
use tokio::sync::Mutex;
use url::Url;

pub trait WorkloadFactory {
    fn create<S: Searcher>(&mut self, engine: Engine<S>) -> Result<Workload<S>, io::Error>;
}

pub struct Factory {
    id: i32,
    url_channel: Receiver<Url>,
    result_channel: Sender<(Vec<Url>, Value)>,
}

impl Factory {
    pub fn new(url_channel: Receiver<Url>, result_channel: Sender<(Vec<Url>, Value)>) -> Self {
        Self {
            id: 0,
            url_channel,
            result_channel,
        }
    }
}

impl WorkloadFactory for Factory {
    fn create<S: Searcher>(&mut self, engine: Engine<S>) -> Result<Workload<S>, io::Error> {
        let id = self.id;
        self.id += 1;

        Ok(Workload::new(
            id,
            engine,
            self.url_channel.clone(),
            self.result_channel.clone(),
        ))
    }
}

// todo: think about a way to have a support of webdrivers
// which doesn't backed by `xenon`.
//
// Where user don't only provides a number of jobs, but
// also a url connection for each job?
//
// todo: config of default URL
async fn create_webdriver(timeout: Duration) -> WebDriver {
    let mut cops = DesiredCapabilities::firefox();
    cops.set_headless().unwrap();

    // by this option we try to resolve CAPTCHAs
    cops.add("unhandledPromptBehavior", "accept").unwrap();

    let driver = WebDriver::new("http://localhost:4444", &cops)
        .await
        .unwrap();
    driver.set_page_load_timeout(timeout).await.unwrap();

    driver
}
