// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    backend::{SideRunner, WebDriverSearcher},
    engine::Engine,
    filters::Filter,
};
use async_trait::async_trait;
use std::{fmt::Display, io, time::Duration};
use thirtyfour::{
    prelude::WebDriverResult, Capabilities, DesiredCapabilities, WebDriver, WebDriverCommands,
};
use url::Url;

#[async_trait]
pub trait EngineBuilder {
    type Backend;

    async fn build(&mut self) -> io::Result<Engine<Self::Backend>>;
}

pub struct WebDriverEngineBuilder {
    config: WebDriverConfig,
    code: String,
    filters: Vec<Filter>,
    id: usize,
}

#[derive(Debug, Clone)]
pub struct WebDriverConfig {
    pub load_timeout: Duration,
    pub browser: Browser,
    pub webdriver_address: Url,
    pub proxy: Option<Proxy>,
}

#[derive(Debug, Clone)]
pub enum Browser {
    Firefox,
    Chrome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proxy {
    Direct,
    Manual(ManualProxy),
    AutoConfig(String),
    AutoDetect,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualProxy {
    Http(String),
    Sock {
        address: String,
        version: u8,
        username: Option<String>,
        password: Option<String>,
    },
}

impl WebDriverEngineBuilder {
    pub fn new(config: WebDriverConfig, code: String, filters: Vec<Filter>) -> Self {
        Self {
            config,
            code,
            filters,
            id: 0,
        }
    }
}

#[async_trait]
impl EngineBuilder for WebDriverEngineBuilder {
    type Backend = WebDriverSearcher;

    async fn build(&mut self) -> io::Result<Engine<Self::Backend>> {
        let wb = create_webdriver(&self.config)
            .await
            .map_err(|e| wrap_err("Failed to create a webdriver", e))?;
        let searcher = WebDriverSearcher::new(wb, self.code.clone());
        let id = self.id;
        self.id += 1;
        let engine = Engine::new(id, searcher, &self.filters);

        Ok(engine)
    }
}

async fn create_webdriver(cfg: &WebDriverConfig) -> WebDriverResult<WebDriver> {
    let driver = match cfg.browser {
        Browser::Firefox => {
            let mut cops = DesiredCapabilities::firefox();
            cops.set_headless()?;
            // by this option we try to resolve CAPTCHAs
            cops.add("unhandledPromptBehavior", "accept")?;

            if let Some(p) = cfg.proxy.as_ref() {
                let proxy = convert_proxy(p);
                cops.set_proxy(proxy)?;
            }

            WebDriver::new_with_timeout(
                cfg.webdriver_address.as_str(),
                &cops,
                Some(Duration::from_millis(3000)),
            )
            .await?
        }
        Browser::Chrome => {
            let mut cops = DesiredCapabilities::chrome();
            cops.set_headless()?;
            // by this option we try to resolve CAPTCHAs
            cops.add("unhandledPromptBehavior", "accept")?;

            if let Some(p) = cfg.proxy.as_ref() {
                let proxy = convert_proxy(p);
                cops.set_proxy(proxy)?;
            }

            WebDriver::new_with_timeout(
                cfg.webdriver_address.as_str(),
                &cops,
                Some(Duration::from_millis(3000)),
            )
            .await?
        }
    };

    driver.set_page_load_timeout(cfg.load_timeout).await?;

    Ok(driver)
}

fn convert_proxy(p: &Proxy) -> thirtyfour::Proxy {
    match p {
        Proxy::Manual(ManualProxy::Sock {
            address,
            password,
            username,
            version,
        }) => thirtyfour::Proxy::Manual {
            socks_proxy: Some(address.to_string()),
            socks_version: Some(*version),
            socks_username: username.clone(),
            socks_password: password.clone(),
            http_proxy: None,
            ssl_proxy: None,
            ftp_proxy: None,
            no_proxy: None,
        },
        Proxy::Manual(ManualProxy::Http(address)) => thirtyfour::Proxy::Manual {
            socks_proxy: None,
            socks_version: None,
            socks_username: None,
            socks_password: None,
            http_proxy: Some(address.to_string()),
            ssl_proxy: Some(address.to_string()),
            ftp_proxy: Some(address.to_string()),
            no_proxy: None,
        },
        Proxy::AutoConfig(url) => thirtyfour::Proxy::AutoConfig {
            url: url.to_string(),
        },
        Proxy::AutoDetect => thirtyfour::Proxy::AutoDetect,
        Proxy::Direct => thirtyfour::Proxy::Direct,
        Proxy::System => thirtyfour::Proxy::System,
    }
}

pub fn wrap_err<S: Into<String>>(msg: S, e: impl Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{} {}", msg.into(), e))
}

pub struct SideRunnerEngineBuilder {
    config: WebDriverConfig,
    code: String,
    filters: Vec<Filter>,
    id: usize,
}

impl SideRunnerEngineBuilder {
    pub fn new(config: WebDriverConfig, code: String, filters: Vec<Filter>) -> Self {
        Self {
            config,
            code,
            filters,
            id: 0,
        }
    }
}

#[async_trait]
impl EngineBuilder for SideRunnerEngineBuilder {
    type Backend = SideRunner;

    async fn build(&mut self) -> io::Result<Engine<Self::Backend>> {
        let wb = create_webdriver(&self.config)
            .await
            .map_err(|e| wrap_err("Failed to create a webdriver", e))?;

        let file = siderunner::parse(std::io::Cursor::new(self.code.clone()))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))?;
        let searcher = SideRunner::new(wb, file);
        let id = self.id;
        self.id += 1;
        let engine = Engine::new(id, searcher, &self.filters);

        Ok(engine)
    }
}
