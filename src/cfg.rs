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
use fancy_regex::Regex;
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
    /// If any of the regex matches a url, it is considered to be ignored.
    /// If nothing is provided it uses a list of regexes which restricts a
    /// everything by domain of provided urls.
    #[clap(short, long)]
    pub ignore: Option<Vec<String>>,
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

        let ignore_list = self
            .ignore_list()
            .context("Failed to parse regexes in an ignore list")?;
        filters.extend(ignore_list);

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
    let mut filters = cfg.filters()?;
    let mut urls = cfg.get_urls()?;
    clean_urls(&mut urls, &filters);
    if filters.is_empty() {
        filters = domain_filters(&urls);
    }

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

fn domain_filters(urls: &[Url]) -> Vec<Filter> {
    urls.iter()
        .cloned()
        .map(|mut url| {
            url.set_query(None);
            url.set_path("");
            url.set_fragment(None);
            url
        })
        .map(|url| Regex::new(&format!("^(?!^{}).*$", url)).unwrap())
        .map(|regex| Filter::Regex(regex))
        .collect()
}

fn parse_urls(strings: &[&str], urls: &mut Vec<Url>) -> Result<(), url::ParseError> {
    for url in strings {
        let url = url::Url::parse(url)?;
        urls.push(url);
    }

    Ok(())
}

fn clean_urls(urls: &mut Vec<Url>, filters: &[Filter]) {
    urls.sort();
    urls.dedup();
    urls.retain(|u| !filters.iter().any(|f| f.is_ignored(u)));
}

fn default_code_file() -> &'static str {
    "return window.location.href"
}
