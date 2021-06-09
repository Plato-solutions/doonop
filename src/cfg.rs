// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    engine_builder::{Browser, WebDriverConfig},
    filters::Filter,
    Code, CodeType, CrawlConfig,
};
use anyhow::{Context, Result};
use clap::Clap;
use regex::RegexSet;
use std::{
    io::{self, Read},
    str::FromStr,
    time::Duration,
};
use url::Url;

const DEFAULT_LOAD_TIME: Duration = Duration::from_secs(10);
const DEFAULT_AMOUNT_OF_ENGINES: usize = 1;

#[derive(Clap)]
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
    /// If any of the regex returns true the url considered to be missed.
    /// It uses [`regex`](https://docs.rs/regex/1.3.9/regex/) so such features as
    /// lookahead and negative lookahead are not available. Mostly in regard of ?perfomance?
    /// (If there will be any demand Might its reasonable to switch to `fancy-regex`, but now
    /// there's a filter for base url check which covers todays needs)
    #[clap(short, long)]
    pub ignore_list: Option<Vec<String>>,
    /// Filters which help to cover limitations of regex.
    /// Curently there's only 1 filter host-name
    ///     *host-name make sure that only urls within one host-name will be checked.
    /// The syntax of filters is filter_name=value
    #[clap(short, long)]
    pub filters: Option<Vec<String>>,
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
    /// A webdriver address.
    #[clap(short, long, default_value = "http://localhost:4444")]
    pub webdriver_url: String,
    /// A site urls from which the process of checking will be started.
    pub urls: Vec<String>,
}

impl Cfg {
    fn filters(&self) -> Result<Vec<Filter>> {
        let mut filters = Vec::new();
        if let Some(set) = self.filters.as_ref() {
            for f in set {
                let filter = parse_filter(f).context("Failed to parse filters")?;
                filters.push(filter);
            }
        }

        let ignore_list = self
            .ignore_list()
            .context("Failed to parse regexes in an ignore list")?;
        if let Some(set) = ignore_list {
            filters.push(Filter::Regex(set));
        };

        Ok(filters)
    }

    fn ignore_list(&self) -> std::result::Result<Option<RegexSet>, regex::Error> {
        self.ignore_list
            .as_ref()
            .map(regex::RegexSet::new)
            .transpose()
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

    fn urls_from_seed_file(&self, urls: &mut Vec<Url>) -> Result<()> {
        match &self.seed_file {
            Some(file) => {
                let mut seed_file = std::fs::File::open(file)?;
                let mut file = String::new();
                seed_file.read_to_string(&mut file)?;
                let lines = file.lines().collect::<Vec<_>>();
                parse_urls(&lines, urls)?;
            }
            None => (),
        }

        Ok(())
    }

    fn urls_from_cfg(&self, urls: &mut Vec<Url>) -> Result<()> {
        let u = self.urls.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        parse_urls(&u, urls)?;
        Ok(())
    }

    fn get_urls(&self) -> Result<Vec<Url>> {
        let mut urls = Vec::new();
        self.urls_from_cfg(&mut urls)
            .context("Failed to get urls from Config")?;
        self.urls_from_seed_file(&mut urls)
            .context("Failed to get urls from a seed file")?;
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

pub fn parse_cfg(cfg: Cfg) -> Result<CrawlConfig> {
    let browser = cfg.browser.clone();
    let wb_address =
        Url::parse(&cfg.webdriver_url).context("Failed to parse a webdriver address")?;
    let page_load_timeout = cfg
        .page_load_timeout
        .map(|milis| Duration::from_millis(milis))
        .unwrap_or_else(|| DEFAULT_LOAD_TIME);
    let amount_searchers = cfg.count_searchers.unwrap_or(DEFAULT_AMOUNT_OF_ENGINES);
    let check_code = cfg
        .open_code_file()
        .context("Failed to read an check file")?;
    let filters = cfg.filters()?;
    let mut urls = cfg.get_urls()?;
    clean_urls(&mut urls, &filters);

    let config = CrawlConfig {
        count_engines: amount_searchers,
        filters: filters,
        url_limit: cfg.limit,
        urls: urls,
        code: Code {
            text: check_code,
            code_type: CodeType::Js,
        },
        wb_config: WebDriverConfig {
            webdriver_address: wb_address,
            browser: browser,
            load_timeout: page_load_timeout,
        },
    };

    Ok(config)
}

pub fn parse_urls(strings: &[&str], urls: &mut Vec<Url>) -> Result<(), url::ParseError> {
    for url in strings {
        let url = url::Url::parse(url)?;
        urls.push(url);
    }

    Ok(())
}

fn parse_filter(s: &str) -> Result<Filter, regex::Error> {
    match s.find('=') {
        Some(pos) => {
            let filter = s[..pos].to_owned();
            let value = s[pos + 1..].to_owned();
            match filter.as_str() {
                "host-name" => {
                    let url =
                        Url::parse(&value).map_err(|ee| regex::Error::Syntax(ee.to_string()))?;
                    Ok(Filter::HostName(url))
                }
                _ => {
                    return Err(regex::Error::Syntax(format!(
                        "unexpected filter name {}",
                        filter
                    )));
                }
            }
        }
        None => {
            return Err(regex::Error::Syntax(
                "filter expected to be splited with '='".to_owned(),
            ));
        }
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
