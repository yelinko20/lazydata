use super::postgres::PostgresExecutor;
use crate::database::pool::DbPool;
use crate::layout::data_table::DynamicData;
use crate::utils::query_type::Query;
use async_trait::async_trait;

pub enum ExecutionResult {
    Data(DynamicData),
    Affected(u64),
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

pub async fn execute_query(pool: &DbPool, sql: &str) -> Result<ExecutionResult, sqlx::Error> {
    let executor = create_executor(pool);
    match Query::from_sql(sql) {
        Query::SELECT => {
            let data = executor.fetch(sql).await?;
            Ok(ExecutionResult::Data(data))
        }
        Query::INSERT => {
            let rows = executor.insert(sql).await?;
            Ok(ExecutionResult::Affected(rows))
        }
        Query::UPDATE => {
            let rows = executor.update(sql).await?;
            Ok(ExecutionResult::Affected(rows))
        }
        Query::DELETE => {
            let rows = executor.delete(sql).await?;
            Ok(ExecutionResult::Affected(rows))
        }
        Query::UNKNOWN => Err(sqlx::Error::Protocol("Unsupported query".into())),
    }
}
