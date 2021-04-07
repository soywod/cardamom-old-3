use chrono::{DateTime, Utc};
use error_chain::error_chain;
use std::{collections::HashMap, fs};

error_chain! {}

use crate::{config::Config, local::model::Card as LocalCard, remote::model::Card as RemoteCard};

pub struct CacheItem {
    pub name: String,
    pub etag: String,
    pub local_date: DateTime<Utc>,
    pub remote_date: DateTime<Utc>,
}

impl ToString for CacheItem {
    fn to_string(&self) -> String {
        format!(
            "{};{};{};{}",
            self.name, self.etag, self.local_date, self.remote_date
        )
    }
}

pub struct Cache {
    pub ctag: String,
    pub cards: HashMap<String, CacheItem>,
}

impl Cache {
    pub fn build_and_write(
        config: &Config,
        ctag: String,
        lcards: HashMap<String, LocalCard>,
        rcards: HashMap<String, RemoteCard>,
    ) -> Result<()> {
        let cards = lcards
            .iter()
            .fold(HashMap::new(), |mut cache, (name, lcard)| {
                match rcards.get(name) {
                    None => cache,
                    Some(rcard) => {
                        cache.insert(
                            name.to_owned(),
                            CacheItem {
                                name: name.to_owned(),
                                etag: rcard.etag.to_owned(),
                                local_date: lcard.date,
                                remote_date: rcard.date,
                            },
                        );
                        cache
                    }
                }
            })
            .iter()
            .map(|(_, card)| card.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(config.file_path(".cache"), format!("{}\n{}", ctag, cards))
            .chain_err(|| "Could not write cache")
    }
}
