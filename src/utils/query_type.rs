#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum Query {
    SELECT,
    INSERT,
    UPDATE,
    DELETE,
    UNKNOWN,
}

impl Query {
    pub fn from_sql(sql: &str) -> Self {
        let trimmed = sql.trim_start().to_uppercase();
        match trimmed.split_whitespace().next() {
            Some("SELECT") => Query::SELECT,
            Some("INSERT") => Query::INSERT,
            Some("UPDATE") => Query::UPDATE,
            Some("DELETE") => Query::DELETE,
            _ => Query::UNKNOWN,
        }
    }
}
