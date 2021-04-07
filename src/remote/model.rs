use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Card {
    pub etag: String,
    pub name: String,
    pub date: DateTime<Utc>,
}
