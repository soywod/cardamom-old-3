use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Card {
    pub name: String,
    pub date: DateTime<Utc>,
}
