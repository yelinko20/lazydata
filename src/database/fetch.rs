use crate::layout::data_table::DynamicData;

use super::pool::DbPool;
use color_eyre::eyre::Result;
use futures::future::try_join_all;
use ratatui::text::Text;
use sqlx::{
    Column, MySqlPool, PgPool, Row, SqlitePool,
    types::{Json, Uuid, chrono},
};

use tui_tree_widget::TreeItem;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TableMetadata {
    pub name: String,
    pub columns: Vec<String>,
    pub constraints: Vec<String>,
    pub indexes: Vec<String>,
    pub rls_policies: Vec<String>,
    pub rules: Vec<String>,
    pub triggers: Vec<String>,
    pub row_count: i64,
    pub estimated_size: String,
    pub table_type: String,
    pub table_data: Option<DynamicData>,
}

#[allow(dead_code)]
pub trait TableMetadataUtils {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

impl TableMetadataUtils for Vec<TableMetadata> {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

#[async_trait::async_trait]
pub trait MetadataFetcher: Send + Sync {
    async fn fetch_metadata(&self) -> Result<Vec<TableMetadata>>;
}

#[async_trait::async_trait]
impl MetadataFetcher for PgPool {
    async fn fetch_metadata(&self) -> Result<Vec<TableMetadata>> {
        let rows = sqlx::query(
            r#"
                SELECT 
                    c.relname AS table_name,
                    CASE 
                        WHEN c.reltuples < 0 THEN 0
                        ELSE c.reltuples::BIGINT
                    END AS row_estimate,
                    pg_size_pretty(pg_total_relation_size(c.oid)) AS total_size,
                    CASE c.relkind 
                        WHEN 'r' THEN 'table'
                        WHEN 'v' THEN 'view'
                        WHEN 'm' THEN 'materialized view'
                        WHEN 'f' THEN 'foreign table'
                        ELSE 'other'
                    END AS table_type
                FROM pg_class c
                JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE n.nspname = 'public' AND c.relkind IN ('r', 'v', 'm', 'f')
                ORDER BY c.relname;
            "#,
        )
        .fetch_all(self)
        .await?;

        let table_futures = rows.into_iter().map(|row| {
            let pool = self.clone();
            async move {
                let table_name: String = row.get("table_name");
                let row_count: i64 = row.get("row_estimate");
                let estimated_size: String = row.get("total_size");
                let table_type: String = row.get("table_type");

                let columns = get_pg_columns(&pool, &table_name).await?;
                let constraints = get_pg_constraints(&pool, &table_name).await?;
                let indexes = get_pg_indexes(&pool, &table_name).await?;
                let rls_policies = get_pg_rls_policies(&pool, &table_name).await?;
                let rules = get_pg_rules(&pool, &table_name).await?;
                let triggers = get_pg_triggers(&pool, &table_name).await?;

                Ok::<_, sqlx::Error>(TableMetadata {
                    name: table_name,
                    columns,
                    constraints,
                    indexes,
                    rls_policies,
                    rules,
                    triggers,
                    row_count,
                    estimated_size,
                    table_type,
                    table_data: None,
                })
            }
        });

        let metadata = try_join_all(table_futures).await?;
        Ok(metadata)
    }
}

#[async_trait::async_trait]
impl MetadataFetcher for MySqlPool {
    async fn fetch_metadata(&self) -> Result<Vec<TableMetadata>> {
        let rows = sqlx::query("SHOW TABLE STATUS").fetch_all(self).await?;

        let mut tables = Vec::new();
        for row in rows {
            let table_name: String = row.get("Name");
            let row_count: i64 = row.try_get("Rows").unwrap_or(0);
            let estimated_size: String = {
                let data_length: i64 = row.try_get("Data_length").unwrap_or(0);
                let index_length: i64 = row.try_get("Index_length").unwrap_or(0);
                format!("{} bytes", data_length + index_length)
            };
            let table_type: String = row.try_get("Comment").unwrap_or("".to_string());

            let columns = sqlx::query(&format!("SHOW COLUMNS FROM `{}`", table_name))
                .fetch_all(self)
                .await?
                .into_iter()
                .map(|r| r.get("Field"))
                .collect();

            let triggers = sqlx::query("SHOW TRIGGERS WHERE `Table` = ?")
                .bind(&table_name)
                .fetch_all(self)
                .await?
                .into_iter()
                .map(|r| r.get("Trigger"))
                .collect();

            tables.push(TableMetadata {
                name: table_name,
                columns,
                constraints: vec![],
                indexes: vec![],
                rls_policies: vec![],
                rules: vec![],
                triggers,
                row_count,
                estimated_size,
                table_type,
                table_data: None,
            });
        }
        Ok(tables)
    }
}

#[async_trait::async_trait]
impl MetadataFetcher for SqlitePool {
    async fn fetch_metadata(&self) -> Result<Vec<TableMetadata>> {
        let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(self)
            .await?;

        let mut tables = Vec::new();
        for row in rows {
            let table_name: String = row.get("name");

            let columns_rows = sqlx::query(&format!("PRAGMA table_info('{}')", table_name))
                .fetch_all(self)
                .await?;
            let columns = columns_rows.iter().map(|r| r.get("name")).collect();

            let indexes_rows = sqlx::query(&format!("PRAGMA index_list('{}')", table_name))
                .fetch_all(self)
                .await?;
            let indexes = indexes_rows.iter().map(|r| r.get("name")).collect();

            let triggers_rows =
                sqlx::query("SELECT name FROM sqlite_master WHERE type='trigger' AND tbl_name=?")
                    .bind(&table_name)
                    .fetch_all(self)
                    .await?;
            let triggers = triggers_rows.iter().map(|r| r.get("name")).collect();

            tables.push(TableMetadata {
                name: table_name,
                columns,
                constraints: vec![],
                indexes,
                rls_policies: vec![],
                rules: vec![],
                triggers,
                row_count: 0,
                estimated_size: "N/A".to_string(),
                table_type: "table".to_string(),
                table_data: None,
            });
        }
        Ok(tables)
    }
}

pub async fn fetch_all_table_metadata(pool: &DbPool) -> Result<Vec<TableMetadata>> {
    let metadata = match pool {
        DbPool::Postgres(pg) => pg.fetch_metadata().await?,
        DbPool::MySQL(mysql) => mysql.fetch_metadata().await?,
        DbPool::SQLite(sqlite) => sqlite.fetch_metadata().await?,
    };

    if metadata.is_empty() {
        println!("No table metadata found.");
    } else {
        println!("Found {} tables.", metadata.len());
    }

    Ok(metadata)
}

async fn get_pg_columns(pool: &PgPool, table: &str) -> sqlx::Result<Vec<String>> {
    let rows = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = $1")
        .bind(table)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("column_name")).collect())
}

async fn get_pg_constraints(pool: &PgPool, table: &str) -> sqlx::Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT constraint_name FROM information_schema.table_constraints WHERE table_name = $1 AND constraint_type != 'CHECK'",
    )
    .bind(table)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.get("constraint_name")).collect())
}

async fn get_pg_indexes(pool: &PgPool, table: &str) -> sqlx::Result<Vec<String>> {
    let rows = sqlx::query("SELECT indexname FROM pg_indexes WHERE tablename = $1")
        .bind(table)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("indexname")).collect())
}

async fn get_pg_rls_policies(pool: &PgPool, table: &str) -> sqlx::Result<Vec<String>> {
    let rows = sqlx::query("SELECT policyname FROM pg_policies WHERE tablename = $1")
        .bind(table)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("policyname")).collect())
}

async fn get_pg_rules(pool: &PgPool, table: &str) -> sqlx::Result<Vec<String>> {
    let rows = sqlx::query("SELECT rulename FROM pg_rules WHERE tablename = $1")
        .bind(table)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("rulename")).collect())
}

async fn get_pg_triggers(pool: &PgPool, table: &str) -> sqlx::Result<Vec<String>> {
    let rows = sqlx::query("SELECT tgname FROM pg_trigger JOIN pg_class ON tgrelid = pg_class.oid WHERE relname = $1 AND NOT tgisinternal")
        .bind(table)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("tgname")).collect())
}

pub fn build_category_node(
    parent: &str,
    label: &str,
    items: &[String],
) -> TreeItem<'static, String> {
    let id = format!("{}_{}", parent, label);
    if items.is_empty() {
        TreeItem::new_leaf(id.clone(), label.to_string())
    } else {
        let children = items
            .iter()
            .map(|item| {
                let child_id = format!("{}_{}", id, item);
                TreeItem::new_leaf(child_id, item.clone())
            })
            .collect();

        TreeItem::new(id, label.to_string(), children).unwrap()
    }
}

pub fn metadata_to_tree_items(metadata: &[TableMetadata]) -> Vec<TreeItem<'static, String>> {
    metadata
        .iter()
        .map(|table| {
            let id = table.name.clone();

            let children = vec![
                build_category_node(&id, "Columns", &table.columns),
                build_category_node(&id, "Constraints", &table.constraints),
                build_category_node(&id, "Indexes", &table.indexes),
                build_category_node(&id, "RLS Policies", &table.rls_policies),
                build_category_node(&id, "Rules", &table.rules),
                build_category_node(&id, "Triggers", &table.triggers),
            ];

            TreeItem::new(
                id.clone(),
                Text::from(format!(
                    "{} ({} row{})",
                    id,
                    table.row_count,
                    if table.row_count == 0 || table.row_count == 1 {
                        ""
                    } else {
                        "s"
                    }
                )),
                children,
            )
            .unwrap()
        })
        .collect()
}

pub async fn fetch_query(pool: &DbPool, query: &str) -> Result<DynamicData, sqlx::Error> {
    match pool {
        DbPool::Postgres(pg_pool) => fetch_query_pg(pg_pool, query).await,
        DbPool::MySQL(mysql_pool) => fetch_query_mysql(mysql_pool, query).await,
        DbPool::SQLite(sqlite_pool) => fetch_query_sqlite(sqlite_pool, query).await,
    }
}

async fn fetch_query_pg(pool: &PgPool, query: &str) -> Result<DynamicData, sqlx::Error> {
    let rows = sqlx::query(query).fetch_all(pool).await?;

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
        .map(|col| col.name().to_string())
        .collect();

    let mut data_rows = Vec::new();
    let mut column_widths: Vec<u16> = headers.iter().map(|h| h.len() as u16).collect();

    for row in rows {
        let mut data_row = Vec::new();
        for (i, _) in headers.iter().enumerate() {
            let value = get_pg_value_as_string(&row, i);
            column_widths[i] = column_widths[i].max(value.len() as u16);
            data_row.push(value);
        }
        data_rows.push(data_row);
    }

    Ok(DynamicData {
        headers,
        rows: data_rows,
        column_widths,
    })
}

async fn fetch_query_mysql(pool: &MySqlPool, query: &str) -> Result<DynamicData, sqlx::Error> {
    let rows = sqlx::query(query).fetch_all(pool).await?;

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
        .map(|col| col.name().to_string())
        .collect();

    let mut data_rows = Vec::new();
    let mut column_widths: Vec<u16> = headers.iter().map(|h| h.len() as u16).collect();

    for row in rows {
        let mut data_row = Vec::new();
        for (i, _) in headers.iter().enumerate() {
            let value = get_mysql_value_as_string(&row, i);
            column_widths[i] = column_widths[i].max(value.len() as u16);
            data_row.push(value);
        }
        data_rows.push(data_row);
    }

    Ok(DynamicData {
        headers,
        rows: data_rows,
        column_widths,
    })
}

async fn fetch_query_sqlite(pool: &SqlitePool, query: &str) -> Result<DynamicData, sqlx::Error> {
    let rows = sqlx::query(query).fetch_all(pool).await?;

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
        .map(|col| col.name().to_string())
        .collect();

    let mut data_rows = Vec::new();
    let mut column_widths: Vec<u16> = headers.iter().map(|h| h.len() as u16).collect();

    for row in rows {
        let mut data_row = Vec::new();
        for (i, _) in headers.iter().enumerate() {
            let value = get_sqlite_value_as_string(&row, i);
            column_widths[i] = column_widths[i].max(value.len() as u16);
            data_row.push(value);
        }
        data_rows.push(data_row);
    }

    Ok(DynamicData {
        headers,
        rows: data_rows,
        column_widths,
    })
}

fn get_pg_value_as_string(row: &sqlx::postgres::PgRow, index: usize) -> String {
    row.try_get::<String, _>(index)
        .or_else(|_| row.try_get::<&str, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<i16, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<i32, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<i64, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<f32, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<f64, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<bool, _>(index).map(|v| v.to_string()))
        // .or_else(|_| row.try_get::<BigDecimal, _>(index).map(|v| v.to_string()))
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

// For MySQL row
fn get_mysql_value_as_string(row: &sqlx::mysql::MySqlRow, index: usize) -> String {
    row.try_get::<String, _>(index)
        .or_else(|_| row.try_get::<i32, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<i64, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<f64, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<bool, _>(index).map(|v| v.to_string()))
        .unwrap_or_else(|_| "<unsupported>".to_string())
}

// For SQLite row
fn get_sqlite_value_as_string(row: &sqlx::sqlite::SqliteRow, index: usize) -> String {
    row.try_get::<String, _>(index)
        .or_else(|_| row.try_get::<i32, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<i64, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<f64, _>(index).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<bool, _>(index).map(|v| v.to_string()))
        .unwrap_or_else(|_| "<unsupported>".to_string())
}
