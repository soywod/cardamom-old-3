use chrono::{DateTime, Utc};
use error_chain::error_chain;
use std::{collections::HashMap, fs, str::FromStr};

error_chain! {
    errors {
        ParseCacheItemNameNotFoundErr
        ParseCacheItemEtagNotFoundErr
        ParseCacheItemLocalDateNotFoundErr
        ParseCacheItemRemoteDateNotFoundErr
    }
}

use crate::{config::Config, local::model::Card as LocalCard, remote::model::Card as RemoteCard};

#[derive(Debug)]
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

impl FromStr for CacheItem {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut tokens = s.split(";");

        Ok(CacheItem {
            name: tokens
                .next()
                .ok_or(ErrorKind::ParseCacheItemNameNotFoundErr)?
                .trim()
                .to_string(),
            etag: tokens
                .next()
                .ok_or(ErrorKind::ParseCacheItemEtagNotFoundErr)?
                .trim()
                .to_string(),
            local_date: tokens
                .next()
                .ok_or(ErrorKind::ParseCacheItemLocalDateNotFoundErr)?
                .parse()
                .chain_err(|| "Could not parse cache item local date")?,
            remote_date: tokens
                .next()
                .ok_or(ErrorKind::ParseCacheItemRemoteDateNotFoundErr)?
                .parse()
                .chain_err(|| "Could not parse cache item remote date")?,
        })
    }
}

#[derive(Debug)]
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

    pub fn from_file(config: &Config) -> Result<Self> {
        let content = fs::read_to_string(config.file_path(".cache"))
            .chain_err(|| "Could not open cache file")?;
        let mut lines = content.lines();
        let ctag = lines.next().unwrap_or_default().to_string();

        Ok(lines.fold(
            Self {
                ctag,
                cards: HashMap::new(),
            },
            |mut cache, line| {
                if let Ok(card) = line.parse::<CacheItem>() {
                    cache.cards.insert(card.name.to_owned(), card);
                }
                cache
            },
        ))
    }
}
