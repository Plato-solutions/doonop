use regex::RegexSet;
use url::Url;

// todo: is there any more filters?
#[derive(Debug, Clone)]
pub enum Filter {
    HostName(Url),
    Regex(RegexSet),
}

impl Filter {
    pub fn is_ignored(&self, url: &Url) -> bool {
        match self {
            Self::Regex(ignore_list) => ignore_list.is_match(url.as_str()),
            Self::HostName(host) => {
                host.domain().map(|h| h.trim_start_matches("www."))
                    == url.domain().map(|h| h.trim_start_matches("www."))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_ignore_url() {
        let regexs = vec![".jpg$", ".png"];
        let filter = Filter::Regex(RegexSet::new(regexs).unwrap());

        let url = Url::parse("http://google.com").unwrap();
        assert_eq!(filter.is_ignored(&url), false);

        let url = Url::parse("http://google.com/image.png").unwrap();
        assert_eq!(filter.is_ignored(&url), true);

        let url = Url::parse("http://google.com/some/thing/second.jpg").unwrap();
        assert_eq!(filter.is_ignored(&url), true);
    }

    #[test]
    fn test_ignore_urls_by_host_name() {
        let hostname = Url::parse("http://google.com/").unwrap();
        let filter = Filter::HostName(hostname);

        let url = Url::parse("http://example.com").unwrap();
        assert_eq!(filter.is_ignored(&url), false);

        let url = Url::parse("http://google.by").unwrap();
        assert_eq!(filter.is_ignored(&url), false);

        let url = Url::parse("http://google.com").unwrap();
        assert_eq!(filter.is_ignored(&url), true);

        let url = Url::parse("http://www.google.com").unwrap();
        assert_eq!(filter.is_ignored(&url), true);

        let url = Url::parse("https://google.com").unwrap();
        assert_eq!(filter.is_ignored(&url), true);
    }
}
