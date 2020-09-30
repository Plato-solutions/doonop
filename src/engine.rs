use crate::filters::Filter;
use crate::shed::{Job, Sheduler};
use log::{debug, info, warn, error};
use regex::RegexSet;
use serde_json::Value;
use std::sync::Arc;
use thirtyfour::prelude::*;
use tokio::sync::Mutex;
use tokio::time::delay_for;
use url::Url;

pub struct Engine {
    pub(crate) id: i32,
    pub(crate) check: String,
    pub(crate) limit: Option<usize>,
    pub(crate) filters: Vec<Filter>,
    pub(crate) driver: WebDriver,
    pub(crate) shed: Arc<Mutex<Sheduler>>,
}

impl Engine {
    pub async fn search(&mut self) -> Vec<String> {
        debug!("start search on engine {}", self.id);

        let mut ext = Vec::new();
        loop {
            let mut guard = self.shed.lock().await;
            let job = guard.get_job(self.id);
            drop(guard);

            match job {
                Job::Search(url) => {
                    match self.examine_url(&mut ext, &url).await {
                        Ok(()) => (),
                        Err(err) => {
                            // don't put link back in the queue because it might be there forever
                            //
                            // todo: to get an ability put it back we have add a counter on a link how much times it was carried out.
                            // which is not as hard the only question do we whan't that ability?
                            match err {
                                thirtyfour::error::WebDriverError::Timeout(..) => {
                                    warn!("engine {}, timeout on processing link {}", self.id, url);
                                }
                                err => {
                                    error!("engine {} url {}, error {}", self.id, url, err);
                                }
                            }
                        }
                    };
                }
                Job::Idle(sleep) => {
                    info!("IDLE engine {}", self.id);
                    delay_for(sleep).await;
                }
                Job::Closed => {
                    info!("engine {} is about to be closed", self.id);
                    break;
                }
            }
        }

        debug!("stop search on engine {}", self.id);

        ext
    }

    async fn examine_url(&mut self, data: &mut Vec<String>, url: &Url) -> WebDriverResult<()> {
        info!("engine {} processing {}", self.id, url);

        let (value, links) = self.search_url(url).await?;
        if !value.is_null() {
            let json = serde_json::to_string(&value).unwrap();
            data.push(json);

            if self.limit.map_or(false, |limit| data.len() > limit) {
                info!("engine {} has reached a limit", self.id);

                let mut guard = self.shed.lock().await;
                guard.close();
            }
        }

        if !links.is_empty() {
            let urls = validate_links(url, &links, &self.filters);

            let mut guard = self.shed.lock().await;
            guard.mark_urls(urls);
            drop(guard);
        }

        Ok(())
    }

    async fn search_url(&mut self, url: &Url) -> WebDriverResult<(Value, Vec<String>)> {
        self.driver.get(url.as_str()).await?;
        let links = self.driver.find_elements(By::Tag("a")).await?;

        let mut new_links = Vec::new();
        for link in links {
            let href = link.get_attribute("href").await;
            match href {
                Ok(href) => {
                    new_links.push(href);
                }
                Err(thirtyfour::error::WebDriverError::StaleElementReference(..)) => continue,
                Err(err) => return Err(err),
            }
        }

        let value = self
            .driver
            .execute_script(&self.check)
            .await?
            .value()
            .clone();

        Ok((value, new_links))
    }

    pub async fn shutdown(self) -> WebDriverResult<()> {
        debug!("engine {} shutdown", self.id);

        self.driver.quit().await
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

fn is_ignored_url(ignore_list: &Option<RegexSet>, url: &Url) -> bool {
    ignore_list
        .as_ref()
        .map_or(false, |set| set.is_match(url.as_str()))
}

struct Proxy {
    // todo
// proxy-policy?
}
