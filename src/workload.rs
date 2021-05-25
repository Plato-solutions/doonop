use log::{error, info, warn};
use serde_json::Value;
use tokio::time::sleep;

use crate::{
    engine::Engine,
    searcher::Searcher,
    shed::{Job, Sheduler},
};

pub struct Workload<B, S> {
    pub(crate) id: i32,
    pub(crate) sheduler: S,
    pub(crate) engine: Engine<B>,
}

impl<B, S> Workload<B, S>
where
    S: Sheduler,
    B: Searcher,
{
    pub fn new(id: i32, engine: Engine<B>, sheduler: S) -> Self {
        Self {
            id,
            engine,
            sheduler,
        }
    }

    pub async fn start(mut self) -> Vec<Value> {
        info!("engine {} is started", self.id);

        let mut collected_data = Vec::new();
        loop {
            let job = self.sheduler.pool(self.id).await;
            match job {
                Job::Search(url) => {
                    let result = self.engine.run(url).await;
                    match result {
                        Ok((urls, data)) => {
                            self.sheduler.seed(urls).await;
                            collected_data.push(data);
                        }
                        Err(err) => {
                            error!("engine {} got error on processing {}", self.id, err);
                        }
                    }
                }
                Job::Idle(duration) => {
                    warn!("engine {} sleeps for {:?}", self.id, duration);
                    sleep(duration).await;
                }
                Job::Closed => {
                    warn!("engine {} is getting closed", self.id);
                    break;
                }
            }
        }

        collected_data
    }
}

#[cfg(test)]
pub mod tests {
    use url::Url;

    use crate::{searcher::SearchResult, shed::PoolSheduler};

    use super::*;

    #[tokio::test]
    async fn engine_search() {
        let mut workload = Workload::new(0, Engine::mock(Vec::new()), PoolSheduler::default());
        let data = workload.start().await;
        assert!(data.is_empty())
    }

    #[tokio::test]
    async fn engine_with_data_search() {
        let mut engine = Engine::mock(vec![
            Ok(SearchResult::new(
                Vec::new(),
                Value::String("Hello Santa".into()),
            )),
            Ok(SearchResult::new(
                Vec::new(),
                Value::Array(vec![10.into(), 20.into()]),
            )),
        ]);
        let mut workload = Workload::new(0, engine, PoolSheduler::default());
        workload
            .sheduler
            .seed(vec![
                Url::parse("http://google.com").unwrap(),
                Url::parse("http://wahoo.com").unwrap(),
            ])
            .await;

        let data = workload.start().await;
        assert_eq!(
            data,
            vec![
                Value::String("Hello Santa".into()),
                Value::Array(vec![10.into(), 20.into()]),
            ]
        )
    }
}
