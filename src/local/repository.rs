use error_chain::error_chain;
use std::{collections::HashMap, fs, path::PathBuf};

use super::model::Card;
use crate::config::Config;

error_chain! {}

pub fn read_cards(config: &Config) -> Result<HashMap<String, Card>> {
    Ok(fs::read_dir(config.sync_dir.to_owned())
        .chain_err(|| "Could not read cards from sync dir")?
        .filter_map(|entry| match entry {
            Err(_) => None,
            Ok(entry) => {
                let is_entry_vcf = entry
                    .path()
                    .extension()
                    .map(|ext| ext == "vcf")
                    .unwrap_or(false);

                if is_entry_vcf {
                    Some(entry)
                } else {
                    None
                }
            }
        })
        .fold(HashMap::new(), |mut cards, entry| {
            match PathBuf::from(entry.path()).file_stem() {
                None => cards,
                Some(name) => {
                    let card = Card {
                        name: name.to_string_lossy().to_string(),
                        date: entry.metadata().unwrap().modified().unwrap().into(),
                    };

                    cards.insert(name.to_string_lossy().to_string(), card);
                    cards
                }
            }
        }))
}
