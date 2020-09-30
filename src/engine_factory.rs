use crate::engine::Engine;
use crate::filters::Filter;
use crate::shed::Sheduler;
use regex::RegexSet;
use std::sync::Arc;
use thirtyfour::prelude::*;
use tokio::sync::Mutex;

pub struct EngineFactory {
    id_counter: i32,
    state: Arc<Mutex<Sheduler>>,
    check_code: String,
    limit: Option<usize>,
    filters: Vec<Filter>,
}

impl EngineFactory {
    pub fn new(
        code: &str,
        limit: &Option<usize>,
        filters: &[Filter],
    ) -> Self {
        Self {
            id_counter: 0,
            state: Arc::default(),
            check_code: code.to_owned(),
            limit: *limit,
            filters: filters.to_vec(),
        }
    }

    pub fn create(&mut self, driver: WebDriver) -> Engine {
        let engine = Engine {
            id: self.id_counter,
            shed: self.state.clone(),
            check: self.check_code.clone(),
            limit: self.limit,
            filters: self.filters.clone(),
            driver,
        };

        self.id_counter += 1;

        engine
    }

    pub fn sheduler(&self) -> Arc<Mutex<Sheduler>> {
        self.state.clone()
    }
}
