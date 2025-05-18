use crate::layout::data_table::DynamicData;

use super::executor::DatabaseExecutor;
use async_trait::async_trait;
use sqlx::{
    Column, PgPool, Row,
    postgres::PgRow,
    types::{Json, Uuid, chrono},
};
pub struct PostgresExecutor {
    pool: PgPool,
}

impl PostgresExecutor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn get_value_as_string(row: &PgRow, index: usize) -> String {
        row.try_get::<String, _>(index)
            .or_else(|_| row.try_get::<&str, _>(index).map(|v| v.to_string()))
            .or_else(|_| row.try_get::<i16, _>(index).map(|v| v.to_string()))
            .or_else(|_| row.try_get::<i32, _>(index).map(|v| v.to_string()))
            .or_else(|_| row.try_get::<i64, _>(index).map(|v| v.to_string()))
            .or_else(|_| row.try_get::<f32, _>(index).map(|v| v.to_string()))
            .or_else(|_| row.try_get::<f64, _>(index).map(|v| v.to_string()))
            .or_else(|_| row.try_get::<bool, _>(index).map(|v| v.to_string()))
            .or_else(|_| row.try_get::<Uuid, _>(index).map(|v| v.to_string()))
            .or_else(|_| {
                row.try_get::<chrono::NaiveDate, _>(index)
                    .map(|v| v.to_string())
            })
            .or_else(|_| {
                row.try_get::<chrono::NaiveDateTime, _>(index)
                    .map(|v| v.to_string())
            })
            .or_else(|_| {
                row.try_get::<chrono::NaiveTime, _>(index)
                    .map(|v| v.to_string())
            })
            .or_else(|_| {
                row.try_get::<chrono::DateTime<chrono::Utc>, _>(index)
                    .map(|v| v.to_string())
            })
            .or_else(|_| {
                row.try_get::<Json<serde_json::Value>, _>(index)
                    .map(|v| v.to_string())
            })
            .or_else(|_| row.try_get::<Vec<u8>, _>(index).map(|v| format!("{:?}", v)))
            .unwrap_or_else(|_| "".to_string())
    }
}

#[async_trait]
impl DatabaseExecutor for PostgresExecutor {
    async fn fetch(&self, query: &str) -> Result<DynamicData, sqlx::Error> {
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        if rows.is_empty() {
            return Ok(DynamicData {
                headers: vec![],
                rows: vec![],
                column_widths: vec![],
            });
        }

        let headers: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let mut column_widths: Vec<u16> = headers.iter().map(|h| h.len() as u16).collect();

        let mut data_rows = Vec::new();
        for row in rows {
            let mut data_row = Vec::new();
            for (i, cw) in column_widths.iter_mut().enumerate() {
                let val = Self::get_value_as_string(&row, i);
                *cw = (*cw).max(val.len() as u16);
                data_row.push(val);
            }
            data_rows.push(data_row);
        }

        Ok(DynamicData {
            headers,
            rows: data_rows,
            column_widths,
        })
    }

    async fn insert(&self, query: &str) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(query).execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    async fn update(&self, query: &str) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(query).execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    async fn delete(&self, query: &str) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(query).execute(&self.pool).await?;
        Ok(res.rows_affected())
    }
}
