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
    // use url::Url;

    // use crate::{searcher::SearchResult, shed::PoolSheduler};

    // use super::*;

    // #[tokio::test]
    // async fn engine_search() {
    //     let mut workload = Workload::new(0, Engine::mock(Vec::new()), PoolSheduler::default());
    //     let data = workload.start().await;
    //     assert!(data.is_empty())
    // }

    // #[tokio::test]
    // async fn engine_with_data_search() {
    //     let mut engine = Engine::mock(vec![
    //         Ok(SearchResult::new(
    //             Vec::new(),
    //             Value::String("Hello Santa".into()),
    //         )),
    //         Ok(SearchResult::new(
    //             Vec::new(),
    //             Value::Array(vec![10.into(), 20.into()]),
    //         )),
    //     ]);
    //     let mut workload = Workload::new(0, engine, PoolSheduler::default());
    //     workload
    //         .sheduler
    //         .seed(vec![
    //             Url::parse("http://google.com").unwrap(),
    //             Url::parse("http://wahoo.com").unwrap(),
    //         ])
    //         .await;

    //     let data = workload.start().await;
    //     assert_eq!(
    //         data,
    //         vec![
    //             Value::String("Hello Santa".into()),
    //             Value::Array(vec![10.into(), 20.into()]),
    //         ]
    //     )
    // }
}
