// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    engine::{Engine, EngineId},
    engine_builder::EngineBuilder,
};
use std::{collections::HashSet, io};

#[derive(Debug)]
pub struct EngineRing<B, EB> {
    free_list: Vec<Engine<B>>,
    usage_list: HashSet<EngineId>,
    cap: usize,
    builder: EB,
}

impl<B, EB> EngineRing<B, EB>
where
    EB: EngineBuilder<Backend = B>,
{
    pub fn new(builder: EB, cap: usize) -> Self {
        Self {
            cap,
            builder,
            free_list: Vec::new(),
            usage_list: HashSet::new(),
        }
    }

    pub async fn obtain(&mut self) -> io::Result<Engine<B>> {
        if let Some(engine) = self.free_list.pop() {
            self.usage_list.insert(engine.id);
            return Ok(engine);
        }

        if self.usage_list.len() >= self.cap {
            panic!(
                "WBRing cap is reached; mustn't never happen as we spawn N engines for N drivers"
            );
        }

        let id = self.usage_list.len();
        let engine = self.builder.build().await?;
        self.usage_list.insert(id);

        Ok(engine)
    }

    pub fn return_back(&mut self, engine: Engine<B>) {
        self.usage_list.remove(&engine.id);
        self.free_list.push(engine);
    }

    pub fn count_engines_in_use(&self) -> usize {
        self.usage_list.len()
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::{
        engine::Engine,
        engine_builder::EngineBuilder,
        engine_ring::EngineRing,
        backend::{BackendError, SearchResult, Backend},
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use tokio::test;
    use url::Url;

    #[test]
    async fn ring() {
        let n = 3;
        let builder = MockBuilder::new(vec![(); n]);
        let mut ring = EngineRing::new(builder, n);

        for i in 0..n {
            assert!(matches!(ring.obtain().await, Ok(engine) if engine.id == i))
        }
    }

    #[test]
    async fn ring_reuse_engine() {
        let n = 3;
        let builder = MockBuilder::new(vec![(); n]);
        let mut ring = EngineRing::new(builder, n);

        assert!(ring.obtain().await.is_ok());
        let engine = ring.obtain().await.unwrap();
        let id = engine.id;
        ring.return_back(engine);
        let engine = ring.obtain().await.unwrap();
        assert_eq!(id, engine.id);
    }

    #[test]
    #[should_panic]
    async fn panic_on_exceeding_cap() {
        let n = 3;
        let builder = MockBuilder::new(vec![(); n]);
        let mut ring = EngineRing::new(builder, n);

        for i in 0..n {
            assert!(matches!(ring.obtain().await, Ok(engine) if engine.id == i))
        }

        // panic here
        ring.obtain().await.unwrap();
    }

    struct MockBuilder {
        backends: Vec<()>,
        id: usize,
    }

    impl MockBuilder {
        fn new(backends: Vec<()>) -> Self {
            Self { backends, id: 0 }
        }
    }

    #[async_trait]
    impl EngineBuilder for MockBuilder {
        type Backend = ();

        async fn build(&mut self) -> io::Result<Engine<Self::Backend>> {
            if self.backends.is_empty() {
                panic!("Build call wasn't expected");
            }

            let backend = self.backends.remove(0);
            let id = self.id;
            self.id += 1;

            Ok(Engine::new(id, backend, &[]))
        }
    }

    #[async_trait]
    impl Backend for () {
        async fn search(&mut self, _: &Url) -> Result<SearchResult, BackendError> {
            Ok(SearchResult::new(vec![], Value::Null))
        }

        async fn close(self) {}
    }
}
