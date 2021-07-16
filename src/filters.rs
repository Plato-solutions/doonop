// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use fancy_regex::Regex;
use url::Url;

#[derive(Debug, Clone)]
pub enum Filter {
    Regex(Regex),
    Domain(Vec<String>),
}

impl Filter {
    pub fn is_ignored(&self, url: &Url) -> bool {
        match self {
            Self::Regex(regex) => regex.is_match(url.as_str()).unwrap(),
            Self::Domain(filter) => url
                .domain()
                .map(|h| {
                    filter
                        .iter()
                        .any(|d| h.trim_start_matches("www.") == d.trim_start_matches("www."))
                })
                .map(|found| !found)
                .unwrap_or(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex() {
        let f = Filter::Regex(Regex::new(".jpg$").unwrap());
        assert_eq!(
            f.is_ignored(&Url::parse("http://google.com").unwrap()),
            false
        );
        assert_eq!(
            f.is_ignored(&Url::parse("http://google.com/image.png").unwrap()),
            false
        );
        assert_eq!(
            f.is_ignored(&Url::parse("http://google.com/some/thing/second.jpg").unwrap()),
            true
        );

        let f = Filter::Regex(Regex::new("^http://google.com").unwrap());
        assert_eq!(
            f.is_ignored(&Url::parse("http://google.com").unwrap()),
            true
        );
        assert_eq!(
            f.is_ignored(&Url::parse("http://google.com/image.png").unwrap()),
            true
        );
        assert_eq!(
            f.is_ignored(&Url::parse("http://microsoft.com").unwrap()),
            false
        );
    }

    #[test]
    fn test_domain() {
        let f = Filter::Domain(vec!["google.com".to_string(), "www.bing.com".to_string()]);
        assert_eq!(
            f.is_ignored(&Url::parse("http://google.com").unwrap()),
            false
        );
        assert_eq!(
            f.is_ignored(&Url::parse("http://google.com/image.png").unwrap()),
            false
        );
        assert_eq!(
            f.is_ignored(&Url::parse("http://bing.com/image.png?asd=13").unwrap()),
            false
        );
        assert_eq!(
            f.is_ignored(&Url::parse("http://yahoo.com").unwrap()),
            true
        );

    }
}
