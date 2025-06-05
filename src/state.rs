use once_cell::sync::Lazy;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct QueryStats {
    pub rows: usize,
    pub elapsed: Duration,
}

pub static GLOBAL_QUERY_STATS: Lazy<RwLock<Option<QueryStats>>> = Lazy::new(|| RwLock::new(None));

pub async fn update_query_stats(rows: usize, elapsed: Duration) {
    let mut stats = GLOBAL_QUERY_STATS.write().await;
    *stats = Some(QueryStats { rows, elapsed })
}

pub async fn get_query_stats() -> Option<QueryStats> {
    let stats = GLOBAL_QUERY_STATS.read().await;
    stats.clone()
}
