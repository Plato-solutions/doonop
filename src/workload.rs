use std::{collections::HashSet, sync::Arc};

use async_channel::{Receiver, Sender};
use log::{error, info, warn};
use serde_json::Value;
use tokio::{sync::Mutex, time::sleep};
use url::Url;

use crate::{engine::Engine, searcher::Searcher, shed::Sheduler};

pub struct Workload<B> {
    pub(crate) id: i32,
    pub(crate) engine: Engine<B>,
    pub(crate) url_channel: Receiver<Url>,
    pub(crate) result_channel: Sender<(Vec<Url>, Value)>,
}

impl<B: Searcher> Workload<B> {
    pub fn new(
        id: i32,
        engine: Engine<B>,
        url_channel: Receiver<Url>,
        result_channel: Sender<(Vec<Url>, Value)>,
    ) -> Self {
        Self {
            id,
            engine,
            url_channel,
            result_channel,
        }
    }

    pub async fn start(mut self) {
        info!("engine {} is started", self.id);

        while let Ok(url) = self.url_channel.recv().await {
            info!("engine {} works on {}", self.id, url);

            let result = self.engine.run(url).await;
            match result {
                Ok((urls, data)) => self.result_channel.send((urls, data)).await.unwrap(),
                Err(err) => {
                    error!("engine {} got error on processing {}", self.id, err);
                }
            }
        }

        info!("engine {} is closed", self.id);

        // it's urgent to call the close method
        // otherwise the connection isn't closed and somehow there's a datarace
        self.engine.backend.close().await;
    }
}

#[cfg(test)]
pub mod tests {
    use std::time::Duration;

    use async_channel::unbounded;
    use url::Url;

    use crate::searcher::SearchResult;

    use super::*;

    #[tokio::test]
    async fn engine_empty() {
        let (result_s, result_r) = unbounded();
        let (url_s, url_r) = unbounded();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            url_s.close();
            tokio::task::yield_now().await;
        });

        let workload = Workload::new(0, Engine::mock(Vec::new()), url_r, result_s);
        workload.start().await;

        handle.await.unwrap();

        assert!(result_r.is_empty())
    }

    #[tokio::test]
    async fn engine_with_data_search() {
        let engine = Engine::mock(vec![
            Ok(SearchResult::new(
                Vec::new(),
                Value::String("Hello Santa".into()),
            )),
            Ok(SearchResult::new(
                Vec::new(),
                Value::Array(vec![10.into(), 20.into()]),
            )),
        ]);

        let (result_s, result_r) = unbounded();
        let (url_s, url_r) = unbounded();

        url_s
            .send(Url::parse("http://google.com").unwrap())
            .await
            .unwrap();
        url_s
            .send(Url::parse("http://wahoo.com").unwrap())
            .await
            .unwrap();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            url_s.close();
            tokio::task::yield_now().await;
        });

        let workload = Workload::new(0, engine, url_r, result_s);
        workload.start().await;

        handle.await.unwrap();

        let results = result_r.recv().await.unwrap();
        assert_eq!(results.1, Value::String("Hello Santa".into()));

        let results = result_r.recv().await.unwrap();
        assert_eq!(results.1, Value::Array(vec![10.into(), 20.into()]));
    }
}
