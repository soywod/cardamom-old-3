use clap::{self, SubCommand};
use error_chain::error_chain;
use reqwest::Client;
use std::env;

use crate::config::Config;
use crate::{cache::Cache, local, remote};

error_chain! {
    links {
        Config(crate::config::Error, crate::config::ErrorKind);
        Cache(crate::cache::Error, crate::cache::ErrorKind);
        LocalRepository(local::repository::Error, local::repository::ErrorKind);
        RemoteRepository(remote::repository::Error, remote::repository::ErrorKind);
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

        let path = remote::repository::addressbook_path(&config, &client).await?;
        let ctag = remote::repository::fetch_ctag(&config, &client, &path).await?;
        let remote_cards =
            remote::repository::fetch_and_write_cards(&config, &client, &path).await?;
        let local_cards = local::repository::read_cards(&config)?;

        Cache::build_and_write(&config, ctag, local_cards, remote_cards)?;
    }

    if let Some(_) = matches.subcommand_matches("sync") {
        let config = Config::from_file()?;
        let cache = Cache::from_file(&config)?;

        println!("{:#?}", cache);
    }

    Ok(())
}
