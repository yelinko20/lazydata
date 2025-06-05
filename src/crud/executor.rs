use super::postgres::PostgresExecutor;
use crate::database::pool::DbPool;
use crate::layout::data_table::DynamicData;
use crate::state::update_query_stats;
use crate::utils::query_timer::query_timer;
use crate::utils::query_type::Query;
use async_trait::async_trait;
use std::convert::TryInto;
use std::time::Duration;

#[allow(dead_code)]
pub struct DataMeta {
    pub rows: usize,
    pub message: String,
}

#[allow(dead_code)]
pub enum ExecutionResult {
    Affected { rows: usize, message: String },
    Data(DynamicData, DataMeta),
}

#[async_trait]
pub trait DatabaseExecutor: Send + Sync {
    async fn fetch(&self, query: &str) -> Result<DynamicData, sqlx::Error>;
    async fn insert(&self, query: &str) -> Result<u64, sqlx::Error>;
    async fn update(&self, query: &str) -> Result<u64, sqlx::Error>;
    async fn delete(&self, query: &str) -> Result<u64, sqlx::Error>;
}

pub fn create_executor(pool: &DbPool) -> Box<dyn DatabaseExecutor + Send + Sync> {
    match pool {
        DbPool::Postgres(pg_pool) => Box::new(PostgresExecutor::new(pg_pool.clone())),
        DbPool::MySQL(_) => todo!(),
        DbPool::SQLite(_) => todo!(),
    }
}

fn format_result_and_update_stats(
    action: &str,
    rows: usize,
    elapsed: Duration,
) -> (ExecutionResult, impl std::future::Future<Output = ()>) {
    let message = format!(
        "{} {} rows affected.\nQuery completed in {} msec.",
        action,
        rows,
        elapsed.as_millis()
    );
    let result = ExecutionResult::Affected {
        rows,
        message: message.clone(),
    };
    let update_future = update_query_stats(rows, elapsed);
    (result, update_future)
}

pub async fn execute_query(pool: &DbPool, sql: &str) -> Result<ExecutionResult, sqlx::Error> {
    let executor = create_executor(pool);

    match Query::from_sql(sql) {
        Query::SELECT => {
            let (rows_result, elapsed) = query_timer(executor.fetch(sql)).await;
            let data = rows_result?;
            let data_len = data.len();

            update_query_stats(data_len, elapsed).await;

            let message = format!(
                "SELECT completed in {} msec.\n{} rows returned.",
                elapsed.as_millis(),
                data_len
            );
            Ok(ExecutionResult::Data(
                data,
                DataMeta {
                    rows: data_len,
                    message,
                },
            ))
        }
        Query::INSERT => {
            let (rows_result, elapsed) = query_timer(executor.insert(sql)).await;
            let rows: usize = rows_result?.try_into().unwrap();
            let (result, update_future) = format_result_and_update_stats("INSERT", rows, elapsed);
            update_future.await;
            Ok(result)
        }
        Query::UPDATE => {
            let (rows_result, elapsed) = query_timer(executor.update(sql)).await;
            let rows: usize = rows_result?.try_into().unwrap();
            let (result, update_future) = format_result_and_update_stats("UPDATE", rows, elapsed);
            update_future.await;
            Ok(result)
        }
        Query::DELETE => {
            let (rows_result, elapsed) = query_timer(executor.delete(sql)).await;
            let rows: usize = rows_result?.try_into().unwrap();
            let (result, update_future) = format_result_and_update_stats("DELETE", rows, elapsed);
            update_future.await;
            Ok(result)
        }
        Query::UNKNOWN => Err(sqlx::Error::Protocol("Unsupported query".into())),
    }
}
