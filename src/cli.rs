use clap::{self, SubCommand};
use error_chain::error_chain;
use reqwest::Client;
use std::env;

use crate::config::Config;
use crate::repository;

error_chain! {
    links {
        Config(crate::config::Error, crate::config::ErrorKind);
    }
}

pub async fn run() -> Result<()> {
    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .subcommand(
            SubCommand::with_name("init")
                .aliases(&["i"])
                .about("Inits local sync dir"),
        )
        .subcommand(
            SubCommand::with_name("sync")
                .aliases(&["s"])
                .about("Synchronizes cards"),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("init") {
        let config = Config::from_file()?;
        let client = Client::new();

        let path = repository::remote::addressbook_path(&config, &client)
            .await
            .chain_err(|| "Could not fetch remote path")?;

        repository::remote::fetch_cards_full(&config, &client, &path)
            .await
            .chain_err(|| "Could not fetch cards")?;

        // println!("{:#?}", path);
    }

    Ok(())
}
