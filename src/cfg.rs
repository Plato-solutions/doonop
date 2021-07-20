// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    engine_builder::{Browser, ManualProxy, Proxy, WebDriverConfig},
    filters::Filter,
    workload::RetryPolicy,
    Code, CodeType, CrawlConfig,
};
use clap::Clap;
use fancy_regex::Regex;
use std::{
    collections::HashMap,
    fmt::Display,
    io::{self, Read},
    str::FromStr,
    time::Duration,
};
use url::Url;

const DEFAULT_LOAD_TIME: Duration = Duration::from_secs(10);
const DEFAULT_AMOUNT_OF_ENGINES: usize = 1;

#[derive(Debug, Clap)]
#[clap(version = "1.0", author = "Maxim Zhiburt <zhiburt@gmail.com>")]
pub struct Cfg {
    /// A path to a Javascript file which considered to return a JSON if the value is different from `null`
    /// it will be saved and present in the output.  
    /// By default it saves a url of a page.
    #[clap(short = 'c', long = "check-file")]
    pub check_file: Option<String>,
    /// An amount of searchers which will be spawned
    #[clap(short = 'j')]
    pub count_searchers: Option<usize>,
    /// Limit of found artifacts
    #[clap(short, long)]
    pub limit: Option<usize>,
    /// A page load timeout after crossing which the searcher will skip the URL.
    /// Value is supposed to be in milliseconds.
    #[clap(short, long)]
    pub page_load_timeout: Option<u64>,
    /// A list of regex which determines which url paths may be ingored.
    /// Usefull for reducing a pool of urls which is up to be checked.
    /// If any of the regex matches a url, it is considered to be ignored.
    #[clap(short, long)]
    pub ignore: Option<Vec<String>>,
    /// Filters can be used to restrict crawling process by exact rules.
    /// For example by `domain`
    /// Example:
    /// `-f "domain=google.com"`
    #[clap(short, long)]
    pub filter: Option<Vec<String>>,
    /// A path to file which used to seed a url pool.
    /// A file must denote the following format `url per line`.
    #[clap(short, long)]
    pub seed_file: Option<String>,
    /// A webdriver type you're suppose to run it against.
    /// The expected options are:
    ///     - firefox
    ///     - chrome
    #[clap(short, long, default_value = "firefox")]
    pub browser: Browser,
    /// A policy for a retry in case of network/timeout issue.
    /// The expected options are:
    ///     - no, no retries
    ///     - first, prioritize urls for retry
    ///     - last, prioritize new urls over ones which might be retried
    #[clap(long, default_value = "first")]
    pub retry_policy: RetryPolicy,
    /// A threshold value im milliseconds after which a retry might happen.
    #[clap(long = "retry_threshold", default_value = "10000")]
    pub retry_threshold_milis: u64,
    /// An amount of retries is allowed for a url.
    #[clap(long, default_value = "3")]
    pub retry_count: usize,
    /// Proxy setting.
    /// An example of format is "sock;address=https://example.net;version=5;password=123;username=qwe".
    /// Available types are "sock", "http", "auto-config", "auto-detect", "direct", "system"
    #[clap(long)]
    pub proxy: Option<String>,
    /// A webdriver address.
    #[clap(short, long, default_value = "http://localhost:4444")]
    pub webdriver_url: String,
    /// An option to turn off or turn on a robots.txt check.
    #[clap(long = "use_robots_txt")]
    pub use_robots_txt: bool,
    /// A robot name which will be used for matching
    /// in robot.txt file if it exists.
    #[clap(long = "robot", default_value = "DoonopRobot")]
    pub robot_name: String,
    /// A site urls from which the process of checking will be started.
    pub urls: Vec<String>,
}

impl Cfg {
    fn filters(&self) -> io::Result<Vec<Filter>> {
        let mut filters = Vec::new();

        let ignore_list = self
            .ignore_list()
            .map_err(|e| wrap_err("Failed to parse regexes in an ignore list", e))?;
        filters.extend(ignore_list);

        let _filters = self._filters()?;
        filters.extend(_filters);

        Ok(filters)
    }

    fn ignore_list(&self) -> std::result::Result<Vec<Filter>, fancy_regex::Error> {
        match &self.ignore {
            Some(ignore_list) => {
                let mut v = Vec::with_capacity(ignore_list.len());
                for s in ignore_list {
                    let regex = Regex::new(s)?;
                    v.push(Filter::Regex(regex));
                }

                Ok(v)
            }
            None => Ok(Vec::new()),
        }
    }

    fn _filters(&self) -> io::Result<Vec<Filter>> {
        match &self.filter {
            Some(filters) => {
                let mut v = Vec::with_capacity(filters.len());
                for s in filters {
                    let filter = parse_filter(s).ok_or_else(|| {
                        io::Error::new(io::ErrorKind::Other, "Failed to parse a filter")
                    })?;

                    v.push(filter);
                }

                //squash domains
                let domains = v.iter().fold(Vec::new(), |mut acc, f| match f {
                    Filter::Domain(f) => {
                        acc.extend(f.clone());
                        acc
                    }
                    _ => acc,
                });
                v = v
                    .into_iter()
                    .filter(|f| !matches!(f, Filter::Domain(..)))
                    .collect();
                v.push(Filter::Domain(domains));

                Ok(v)
            }
            None => Ok(Vec::new()),
        }
    }

    fn open_code_file(&self) -> io::Result<String> {
        match &self.check_file {
            Some(path) => {
                let mut check_file = std::fs::File::open(path)?;
                let mut content = String::new();
                check_file.read_to_string(&mut content)?;

                Ok(content)
            }
            None => Ok(default_code_file().to_string()),
        }
    }

    fn urls_from_seed_file(&self, urls: &mut Vec<Url>) -> io::Result<()> {
        match &self.seed_file {
            Some(file) => {
                let mut seed_file = std::fs::File::open(file)?;
                let mut file = String::new();
                seed_file.read_to_string(&mut file)?;
                let lines = file.lines().collect::<Vec<_>>();
                parse_urls(&lines, urls).map_err(|e| wrap_err("", e))?;
            }
            None => (),
        }

        Ok(())
    }

    fn urls_from_cfg(&self, urls: &mut Vec<Url>) -> io::Result<()> {
        let u = self.urls.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        parse_urls(&u, urls).map_err(|e| wrap_err("", e))?;
        Ok(())
    }

    fn get_urls(&self) -> io::Result<Vec<Url>> {
        let mut urls = Vec::new();
        self.urls_from_cfg(&mut urls)
            .map_err(|e| wrap_err("Failed to get urls from Config", e))?;
        self.urls_from_seed_file(&mut urls)
            .map_err(|e| wrap_err("Failed to get urls from a seed file", e))?;
        Ok(urls)
    }
}

impl FromStr for Browser {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Firefox" | "firefox" | "geckodriver" => Ok(Self::Firefox),
            "Chrome" | "chrome" | "chromedriver" => Ok(Self::Chrome),
            _ => Err(""),
        }
    }
}

impl FromStr for RetryPolicy {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "no" | "No" | "off" | "Off" => Ok(Self::No),
            "first" | "First" => Ok(Self::RetryFirst),
            "last" | "Last" => Ok(Self::RetryLast),
            _ => Err(""),
        }
    }
}

pub fn parse_cfg(cfg: Cfg) -> io::Result<CrawlConfig> {
    let browser = cfg.browser.clone();
    let wb_address = Url::parse(&cfg.webdriver_url)
        .map_err(|e| wrap_err("Failed to parse a webdriver address", e))?;
    let page_load_timeout = cfg
        .page_load_timeout
        .map(Duration::from_millis)
        .unwrap_or_else(|| DEFAULT_LOAD_TIME);
    let amount_searchers = cfg.count_searchers.unwrap_or(DEFAULT_AMOUNT_OF_ENGINES);
    let check_code = cfg
        .open_code_file()
        .map_err(|e| wrap_err("Failed to read an check file", e))?;
    let retry_policy = cfg.retry_policy;
    let retry_fire = Duration::from_millis(cfg.retry_threshold_milis);
    let retry_count = cfg.retry_count;
    let proxy = if let Some(proxy) = cfg.proxy.as_ref() {
        let p = parse_proxy(proxy).ok_or_else(|| wrap_err("Failed to parse proxy setting", ""))?;
        Some(p)
    } else {
        None
    };
    let filters = cfg.filters()?;
    let mut urls = cfg.get_urls()?;
    clean_urls(&mut urls, &filters);

    let config = CrawlConfig {
        count_engines: amount_searchers,
        filters,
        url_limit: cfg.limit,
        urls,
        retry_count,
        retry_policy,
        retry_threshold: retry_fire,
        robot_name: cfg.robot_name,
        use_robots_txt: cfg.use_robots_txt,
        code: Code {
            text: check_code,
            code_type: CodeType::Js,
        },
        wb_config: WebDriverConfig {
            webdriver_address: wb_address,
            browser,
            load_timeout: page_load_timeout,
            proxy,
        },
    };

    Ok(config)
}

fn parse_urls(strings: &[&str], urls: &mut Vec<Url>) -> Result<(), url::ParseError> {
    for url in strings {
        let url = url::Url::parse(url)?;
        urls.push(url);
    }

    Ok(())
}

fn parse_proxy(s: &str) -> Option<Proxy> {
    let v = s.split_terminator(';').collect::<Vec<_>>();
    let mut map = HashMap::new();
    for i in 1..v.len() {
        let (left, right) = v.get(i)?.split_once("=")?;
        map.insert(left, right);
    }

    match *(v.first()?) {
        "sock" => Some(Proxy::Manual(ManualProxy::Sock {
            address: map.get("address")?.to_string(),
            password: map.get("password").map(|s| s.to_string()),
            username: map.get("username").map(|s| s.to_string()),
            version: map.get("version")?.parse().ok()?,
        })),
        "http" => {
            let address = map.get("address")?.to_string();
            Some(Proxy::Manual(ManualProxy::Http(address)))
        }
        "auto-config" => {
            let address = map.get("address")?.to_string();
            Some(Proxy::AutoConfig(address))
        }
        "auto-detect" => Some(Proxy::AutoDetect),
        "direct" => Some(Proxy::Direct),
        "system" => Some(Proxy::System),
        _ => None,
    }
}

fn parse_filter(s: &str) -> Option<Filter> {
    let (name, value) = s.split_once('=')?;
    match name {
        "domain" => Some(Filter::Domain(vec![value.to_owned()])),
        _ => None,
    }
}

fn clean_urls(urls: &mut Vec<Url>, filters: &[Filter]) {
    urls.sort();
    urls.dedup();
    urls.retain(|u| !filters.iter().any(|f| f.is_ignored(u)));
}

fn default_code_file() -> &'static str {
    "return window.location.href"
}

pub fn wrap_err<S: Into<String>>(msg: S, e: impl Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{} {}", msg.into(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proxy_test() {
        assert_eq!(parse_proxy("auto-detect"), Some(Proxy::AutoDetect));
        assert_eq!(parse_proxy("direct"), Some(Proxy::Direct));
        assert_eq!(parse_proxy("system"), Some(Proxy::System));
        assert_eq!(
            parse_proxy("auto-config;address=https://example.net"),
            Some(Proxy::AutoConfig("https://example.net".to_string()))
        );
        assert_eq!(
            parse_proxy("auto-config;address=https://example.net;"),
            Some(Proxy::AutoConfig("https://example.net".to_string()))
        );
        assert_eq!(
            parse_proxy("http;address=https://example.net"),
            Some(Proxy::Manual(ManualProxy::Http(
                "https://example.net".to_string()
            )))
        );
        assert_eq!(
            parse_proxy("sock;address=https://example.net;version=5"),
            Some(Proxy::Manual(ManualProxy::Sock {
                address: "https://example.net".to_string(),
                version: 5,
                password: None,
                username: None,
            }))
        );
        assert_eq!(
            parse_proxy("sock;address=https://example.net;version=5;password=123;username=qwe"),
            Some(Proxy::Manual(ManualProxy::Sock {
                address: "https://example.net".to_string(),
                version: 5,
                password: Some("123".to_string()),
                username: Some("qwe".to_string()),
            }))
        );
        assert_eq!(parse_proxy("sock;address=https://example.net"), None);
        assert_eq!(parse_proxy("http;"), None);
        assert_eq!(parse_proxy("http"), None);
    }
}
