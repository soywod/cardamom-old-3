use clap::{self, SubCommand};
use error_chain::error_chain;
use quick_xml::de as xml;
use reqwest::{Client, Method};
use serde::Deserialize;
use std::env;

use crate::config::Config;

error_chain! {
    links {
        Config(crate::config::Error, crate::config::ErrorKind);
    }
}

// Common structs

#[derive(Debug, Deserialize)]
struct Multistatus<T> {
    #[serde(rename = "response")]
    pub responses: Vec<Response<T>>,
}

#[derive(Debug, Deserialize)]
struct Response<T> {
    pub href: Href,
    pub propstat: Propstat<T>,
}

#[derive(Debug, Deserialize)]
struct Propstat<T> {
    pub prop: T,
    pub status: Option<Status>,
}

#[derive(Debug, Deserialize)]
struct Href {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
struct Status {
    #[serde(rename = "$value")]
    pub value: String,
}

// Current user principal structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct CurrentUserPrincipalProp {
    pub current_user_principal: CurrentUserPrincipal,
}

#[derive(Debug, Deserialize)]
struct CurrentUserPrincipal {
    pub href: Href,
}

// Addressbook home set structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AddressbookHomeSetProp {
    pub addressbook_home_set: AddressbookHomeSet,
}

#[derive(Debug, Deserialize)]
struct AddressbookHomeSet {
    pub href: Href,
}

// Addressbooks structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AddressbookProp {
    pub resourcetype: AddressbookResourceType,
}

#[derive(Debug, Deserialize)]
struct AddressbookResourceType {
    pub addressbook: Option<Addressbook>,
}

#[derive(Debug, Deserialize)]
struct Addressbook {}

// Utils

fn propfind() -> Result<Method> {
    Method::from_bytes(b"PROPFIND").chain_err(|| "Could not create custo method PROPFIND")
}

// Run

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
        let scheme = if config.ssl() { "https" } else { "http" };
        let url = format!("{}://{}:{}", &scheme, &config.host, &config.port);

        // Current user principal

        let mut path = String::from("/");
        let res = client
            .request(propfind()?, format!("{}{}", &url, &path))
            .basic_auth(
                &config.login,
                Some(
                    config
                        .passwd()
                        .chain_err(|| "Could not retrieve password")?,
                ),
            )
            .body(
                r#"
                <D:propfind xmlns:D="DAV:">
                    <D:prop>
                        <D:current-user-principal />
                    </D:prop>
                </D:propfind>
                "#,
            )
            .send()
            .await
            .chain_err(|| "Could not send request")?;
        let res = res
            .text()
            .await
            .chain_err(|| "Could not extract text body from response")?;
        let res: Multistatus<CurrentUserPrincipalProp> =
            xml::from_str(&res).chain_err(|| "Cannot parse current user principal response")?;

        path = res
            .responses
            .first()
            .map(|res| {
                res.propstat
                    .prop
                    .current_user_principal
                    .href
                    .value
                    .to_owned()
            })
            .unwrap_or(path);

        // Addressbook home set

        let res = client
            .request(propfind()?, format!("{}{}", &url, &path))
            .basic_auth(
                &config.login,
                Some(
                    config
                        .passwd()
                        .chain_err(|| "Could not retrieve password")?,
                ),
            )
            .body(
                r#"
                <D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
                    <D:prop>
                        <C:addressbook-home-set />
                    </D:prop>
                </D:propfind>
                "#,
            )
            .send()
            .await
            .chain_err(|| "Could not send request")?;
        let res = res
            .text()
            .await
            .chain_err(|| "Could not extract text body from response")?;
        let res: Multistatus<AddressbookHomeSetProp> =
            xml::from_str(&res).chain_err(|| "Cannot parse addressbook home set response")?;

        path = res
            .responses
            .first()
            .map(|res| res.propstat.prop.addressbook_home_set.href.value.to_owned())
            .unwrap_or(path);

        // Default addressbook

        let res = client
            .request(propfind()?, format!("{}{}", &url, &path))
            .basic_auth(
                &config.login,
                Some(
                    config
                        .passwd()
                        .chain_err(|| "Could not retrieve password")?,
                ),
            )
            .send()
            .await
            .chain_err(|| "Could not send request")?;
        let res = res
            .text()
            .await
            .chain_err(|| "Could not extract text body from response")?;
        let res: Multistatus<AddressbookProp> =
            xml::from_str(&res).chain_err(|| "Cannot parse addressbook response")?;

        path = res
            .responses
            .iter()
            .find(|res| {
                let valid_status = res
                    .propstat
                    .status
                    .as_ref()
                    .map(|s| s.value.ends_with("200 OK"))
                    .unwrap_or(false);
                let has_addressbook = res
                    .propstat
                    .prop
                    .resourcetype
                    .addressbook
                    .as_ref()
                    .is_some();

                valid_status && has_addressbook
            })
            .map(|res| res.href.value.to_owned())
            .unwrap_or(path);

        println!("{:#?}", path);
    }

    Ok(())
}
