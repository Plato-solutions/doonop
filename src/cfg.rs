// todo: the main function and this module still looks freaky and not in complete state.
//      at least `.unwrap()`
//
// todo: error handling?
use crate::filters::Filter;
use clap::Clap;
use regex::RegexSet;
use std::io::{self, Read};
use url::Url;

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
    /// A site urls from which the process of checking will be started.
    pub urls: Vec<String>,
}

impl Cfg {
    pub fn filters(&self) -> Result<Vec<Filter>, regex::Error> {
        let mut filters = Vec::new();

        // todo: handle removing of filter
        // because there might be usefull to have a default filters which could be removed
        if let Some(set) = self.filters.as_ref() {
            for f in set {
                let filter = match f.find('=') {
                    Some(pos) => {
                        let filter = f[..pos].to_owned();
                        let value = f[pos + 1..].to_owned();
                        match filter.as_str() {
                            "host-name" => {
                                let url = Url::parse(&value)
                                    .map_err(|ee| regex::Error::Syntax(ee.to_string()))?;
                                Filter::HostName(url)
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
                };

                filters.push(filter);
            }
        }

        let ignore_list = self.ignore_list()?;
        if let Some(set) = ignore_list {
            filters.push(Filter::Regex(set));
        };

        Ok(filters)
    }

    pub fn ignore_list(&self) -> Result<Option<RegexSet>, regex::Error> {
        self.ignore_list
            .as_ref()
            .map(regex::RegexSet::new)
            .transpose()
    }

    pub fn open_code_file(&self) -> io::Result<String> {
        match &self.check_file {
            Some(path) => {
                let mut check_file = std::fs::File::open(path)?;
                let mut content = String::new();
                check_file.read_to_string(&mut content).unwrap();

                Ok(content)
            }
            None => Ok(default_code_file().to_string()),
        }
    }

    pub fn urls_from_seed_file(&self, urls: &mut Vec<Url>) -> io::Result<()> {
        match &self.seed_file {
            Some(file) => {
                let mut seed_file = std::fs::File::open(file).expect("a seed file cann't be open");
                let mut file = String::new();
                seed_file.read_to_string(&mut file).unwrap();

                for line in file.lines() {
                    let url = url::Url::parse(line).expect("unexpected type of url");
                    assert!(url.domain().is_some());

                    urls.push(url);
                }
            }
            None => (),
        }

        Ok(())
    }

    pub fn urls_from_cfg(&self, urls: &mut Vec<Url>) -> io::Result<()> {
        for url in &self.urls {
            let url = url::Url::parse(url).expect("unexpected type of url");
            assert!(url.domain().is_some());

            urls.push(url);
        }

        Ok(())
    }
}

fn default_code_file() -> &'static str {
    "return window.location.href"
}
