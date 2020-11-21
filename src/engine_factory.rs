use crate::engine::Engine;
use crate::filters::Filter;
use crate::searcher::{Searcher, WebDriverSearcher};
use crate::shed::Sheduler;
use std::sync::Arc;
use thirtyfour::WebDriver;
use tokio::sync::Mutex;

pub struct EngineFactory {
    id_counter: i32,
    state: Arc<Mutex<Sheduler>>,
    check_code: String,
    limit: Option<usize>,
    filters: Vec<Filter>,
}

impl EngineFactory {
    pub fn new(code: &str, limit: &Option<usize>, filters: &[Filter]) -> Self {
        Self {
            id_counter: 0,
            state: Arc::default(),
            check_code: code.to_owned(),
            limit: *limit,
            filters: filters.to_vec(),
        }
    }

    pub fn create<S>(&mut self, searcher: S) -> Engine<S>
    where
        S: Searcher,
    {
        let engine = Engine {
            id: self.id_counter,
            shed: self.state.clone(),
            limit: self.limit,
            filters: self.filters.clone(),
            searcher,
        };

        self.id_counter += 1;

        engine
    }

    pub fn create_webdriver_engine(&mut self, wb: WebDriver) -> Engine<WebDriverSearcher> {
        let engine = Engine {
            id: self.id_counter,
            shed: self.state.clone(),
            limit: self.limit,
            filters: self.filters.clone(),
            searcher: WebDriverSearcher::new(wb, self.check_code.clone()),
        };

        self.id_counter += 1;

        engine
    }

    pub fn sheduler(&self) -> Arc<Mutex<Sheduler>> {
        self.state.clone()
    }
}
