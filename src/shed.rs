/// The module contains a shared state logic for engines.
/// How they get a link on check.
///
/// # Design
///
/// Sheduler is an internal state itself.
/// Why there wasn't used channels in order to remove Idle logic from engines?
/// Because it would mean that sheduler have to work concurently itself.
/// But what is more important that it would require implementing some logic how to balance engines.
/// Why? Becouse we have list of urls in wait_list which must be checked and we can't blindly split the list equally.
/// We also can't have a anlimited channels because by the same reason.
/// Limited channel would may block sometimes. Which denotes spliting state and sheduler.
///
/// Overall it might be not a bad idea but this is how things are done now.
use crate::engine::Engine;
use log;
use regex::RegexSet;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::prelude::*;
use tokio::sync::Mutex;
use url::Url;

/// Sheduler responsible for providing engines with *work*
///
/// Mainly the sheduler abstraction is developed in order to have an ability to identify that
/// To identifying that there's no more work.
/// We could check queeues but we could't guaranteee that some engine was doing work at the time.
/// And it's results could expand a state queues.
///
/// todo: do we need to develop a restore mechanism in case of engine error?
/// now not becouse engine is responsible for its errors but?
#[derive(Default)]
pub struct Sheduler {
    engines: HashMap<i32, EngineState>,
    state: State,
    engines_stoped: bool,
}

#[derive(Default, Debug)]
pub struct State {
    seen_list: HashSet<Url>,
    // in_progress: HashSet<Url>,
    wait_list: Vec<Url>,
}

impl State {
    pub fn update(&mut self, urls: Vec<Url>) {
        for url in urls {
            self.update_url(url)
        }
    }

    pub fn update_url(&mut self, url: Url) {
        if !self.seen_list.contains(&url) {
            self.wait_list.push(url.clone());
            self.seen_list.insert(url);
        }
    }
}

pub enum Job {
    Search(Url),
    Idle(Duration),
    Closed,
}

// todo: might engine initiate a close?
#[derive(PartialEq, Eq)]
pub enum EngineState {
    Idle,
    // could hold a URL for recovery if there would be an error
    Work,
    Created,
}

impl Sheduler {
    pub fn get_job(&mut self, engine_id: i32) -> Job {
        // todo: does this method is too compex?
        // keeping a lock for too long is might a design smell

        if self.is_closed() {
            return Job::Closed;
        }

        if self.engines.iter().all(|(_, s)| s == &EngineState::Idle)
            && self.state.wait_list.is_empty()
        {
            self.close();
            return Job::Closed;
        }

        let url = self.state.wait_list.pop();
        match url {
            Some(url) => {
                self.set_engine_state(engine_id, EngineState::Work);
                Job::Search(url)
            }
            None => {
                self.set_engine_state(engine_id, EngineState::Idle);
                // todo: some logic with dynamic duration?
                Job::Idle(Duration::from_millis(5000))
            }
        }
    }

    pub fn mark_urls(&mut self, urls: Vec<Url>) {
        self.state.update(urls);
    }

    pub fn mark_url(&mut self, url: Url) {
        self.state.update_url(url);
    }

    pub fn is_closed(&self) -> bool {
        self.engines_stoped
    }

    pub fn close(&mut self) {
        self.engines_stoped = true;
    }

    fn set_engine_state(&mut self, id: i32, state: EngineState) {
        self.engines.insert(id, state);
    }
}
