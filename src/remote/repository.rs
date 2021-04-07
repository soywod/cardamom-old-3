use chrono::{DateTime, Utc};
use error_chain::error_chain;
use quick_xml::de as xml;
use reqwest::{Client, Method};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};

use super::model::Card;
use crate::config::Config;

error_chain! {}

// Common structs

#[derive(Debug, Deserialize)]
pub struct Multistatus<T> {
    #[serde(rename = "response")]
    pub responses: Vec<Response<T>>,
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub href: Href,
    pub propstat: Propstat<T>,
}

#[derive(Debug, Deserialize)]
pub struct Propstat<T> {
    pub prop: T,
    pub status: Option<Status>,
}

#[derive(Debug, Deserialize)]
pub struct Href {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Etag {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct LastModified {
    #[serde(with = "date_parser", rename = "$value")]
    pub value: DateTime<Utc>,
}

mod date_parser {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc2822(&s)
            .map(|d| d.into())
            .map_err(serde::de::Error::custom)
    }
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

// Addressbook structs

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

// Address data structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddressDataProp {
    pub address_data: AddressData,
    pub getetag: Etag,
    pub getlastmodified: LastModified,
}

#[derive(Debug, Deserialize)]
pub struct AddressData {
    #[serde(rename = "$value")]
    pub value: String,
}

// Methods

fn propfind() -> Result<Method> {
    Method::from_bytes(b"PROPFIND").chain_err(|| "Could not create custom method PROPFIND")
}

fn report() -> Result<Method> {
    Method::from_bytes(b"REPORT").chain_err(|| "Could not create custom method REPORT")
}

// Fetch URL fns

async fn fetch_current_user_principal_url(
    config: &Config,
    client: &Client,
    path: String,
) -> Result<String> {
    let res = client
        .request(propfind()?, &config.url(&path))
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
        .chain_err(|| "Could not send current user principal request")?;
    let res = res
        .text()
        .await
        .chain_err(|| "Could not extract text body from current user principal response")?;
    let res: Multistatus<CurrentUserPrincipalProp> =
        xml::from_str(&res).chain_err(|| "Could not parse current user principal response")?;

    Ok(res
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
        .unwrap_or(path))
}

async fn fetch_addressbook_home_set_url(
    config: &Config,
    client: &Client,
    path: String,
) -> Result<String> {
    let res = client
        .request(propfind()?, &config.url(&path))
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
        .chain_err(|| "Could not send addressbook home set request")?;
    let res = res
        .text()
        .await
        .chain_err(|| "Could not extract text body from addressbook home set response")?;
    let res: Multistatus<AddressbookHomeSetProp> =
        xml::from_str(&res).chain_err(|| "Could not parse addressbook home set response")?;

    Ok(res
        .responses
        .first()
        .map(|res| res.propstat.prop.addressbook_home_set.href.value.to_owned())
        .unwrap_or(path))
}

async fn fetch_addressbook_url(config: &Config, client: &Client, path: String) -> Result<String> {
    let res = client
        .request(propfind()?, &config.url(&path))
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
        .chain_err(|| "Could not send addressbook request")?;
    let res = res
        .text()
        .await
        .chain_err(|| "Could not extract text body from addressbook response")?;
    let res: Multistatus<AddressbookProp> =
        xml::from_str(&res).chain_err(|| "Could not parse addressbook response")?;

    Ok(res
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
        .unwrap_or(path))
}

pub async fn fetch_and_write_cards(
    config: &Config,
    client: &Client,
    path: &str,
) -> Result<HashMap<String, Card>> {
    let res = client
        .request(report()?, &config.url(&path))
        .basic_auth(
            &config.login,
            Some(
                config
                    .passwd()
                    .chain_err(|| "Could not retrieve password")?,
            ),
        )
        .header("Depth", "1")
        .body(
            r#"
            <C:addressbook-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
                <D:prop>
                    <D:getetag />
                    <D:getlastmodified />
                    <C:address-data />
                </D:prop>
            </C:addressbook-query>
            "#,
        )
        .send()
        .await
        .chain_err(|| "Could not send address data request")?;

    let res = res
        .text()
        .await
        .chain_err(|| "Could not extract text body from address data response")?;
    let res: Multistatus<AddressDataProp> =
        xml::from_str(&res).chain_err(|| "Could not parse address data response")?;

    let cards = res
        .responses
        .iter()
        .filter_map(|res| {
            let card = Card {
                etag: res.propstat.prop.getetag.value.to_owned(),
                name: PathBuf::from(&res.href.value)
                    .file_stem()?
                    .to_string_lossy()
                    .to_string(),
                date: res.propstat.prop.getlastmodified.value,
            };

            let path = config.file_path(&format!("{}.vcf", &card.name));
            let content = res.propstat.prop.address_data.value.trim_end_matches("\r");
            fs::write(&path, &content).ok()?;

            Some(card)
        })
        .fold(HashMap::new(), |mut cards, card| {
            cards.insert(card.name.to_owned(), card);
            cards
        });

    Ok(cards)
}

pub async fn addressbook_path(config: &Config, client: &Client) -> Result<String> {
    let path = String::from("/");
    let path = fetch_current_user_principal_url(&config, &client, path).await?;
    let path = fetch_addressbook_home_set_url(&config, &client, path).await?;
    let path = fetch_addressbook_url(&config, &client, path).await?;

    Ok(path)
}
