use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, SystemTime},
};
use url::Url;

pub struct RetryPool {
    fire_time: Duration,
    count_retries: usize,
    pool: BTreeMap<SystemTime, Vec<Url>>,
    retry_count: HashMap<Url, usize>,
}

impl RetryPool {
    pub fn new(fire_time: Duration, count_retries: usize) -> Self {
        Self {
            fire_time,
            count_retries,
            pool: BTreeMap::new(),
            retry_count: HashMap::new(),
        }
    }

    pub fn keep_retry(&mut self, url: Url) -> bool {
        let count = self.retry_count.entry(url.clone()).or_insert(0);
        *count += 1;
        if *count >= self.count_retries {
            return false;
        }

        let now = SystemTime::now();
        let e = self.pool.entry(now).or_default();
        e.push(url);

        true
    }

    pub fn get_url(&mut self, force: bool) -> Option<Url> {
        // get the most close to be ready url
        let key = self
            .pool
            .keys()
            .next()
            .filter(|time| time.elapsed().unwrap() > self.fire_time || force)
            .cloned();

        match key {
            Some(time) => match self.pool[&time].len() {
                0 => None,
                1 => Some(self.pool.remove(&time).unwrap().pop().unwrap()),
                _ => self.pool.get_mut(&time).unwrap().pop(),
            },
            None => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        let mut pool = RetryPool::new(Duration::new(0, 0), 2);
        pool.keep_retry(Url::parse("https://example_1.net").unwrap());
        pool.keep_retry(Url::parse("https://example_2.net").unwrap());
        pool.keep_retry(Url::parse("https://example_3.net").unwrap());
        assert_eq!(
            pool.get_url(false),
            Some(Url::parse("https://example_1.net").unwrap())
        );
        assert_eq!(
            pool.get_url(false),
            Some(Url::parse("https://example_2.net").unwrap())
        );
        assert_eq!(
            pool.get_url(false),
            Some(Url::parse("https://example_3.net").unwrap())
        );
        assert_eq!(pool.get_url(false), None);
    }

    #[test]
    fn get_with_fire() {
        let mut pool = RetryPool::new(Duration::from_millis(50), 2);
        pool.keep_retry(Url::parse("https://example_1.net").unwrap());
        assert_eq!(pool.get_url(false), None);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            pool.get_url(false),
            Some(Url::parse("https://example_1.net").unwrap())
        );
        assert_eq!(pool.get_url(false), None);
    }

    #[test]
    fn get_count_retries() {
        let mut pool = RetryPool::new(Duration::default(), 3);

        for _ in 0..2 {
            let is_not_over = pool.keep_retry(Url::parse("https://example_1.net").unwrap());
            assert_eq!(is_not_over, true);
            assert_eq!(
                pool.get_url(false),
                Some(Url::parse("https://example_1.net").unwrap())
            );
        }

        let is_not_over = pool.keep_retry(Url::parse("https://example_1.net").unwrap());
        assert_eq!(is_not_over, false);
        assert_eq!(pool.get_url(false), None);
    }

    #[test]
    fn get_force() {
        let mut pool = RetryPool::new(Duration::from_millis(50), 2);
        pool.keep_retry(Url::parse("https://example_1.net").unwrap());
        assert_eq!(
            pool.get_url(true),
            Some(Url::parse("https://example_1.net").unwrap())
        );
        assert_eq!(pool.get_url(false), None);
    }
}
