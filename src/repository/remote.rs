use error_chain::error_chain;
use quick_xml::de as xml;
use reqwest::{Client, Method};
use serde::Deserialize;

use crate::config::Config;

error_chain! {}

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

pub async fn fetch_cards_full(config: &Config, client: &Client, path: &str) -> Result<()> {
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
        .header("Depth", 1)
        .body(
            r#"
            <C:addressbook-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
                <D:prop>
                    <D:href />
                    <D:getetag />
                    <D:getlastmodified />
                    <C:address-data />
                </D:prop>
            </C:addressbook-query>
            "#,
        )
        .send()
        .await
        .chain_err(|| "Could not send addressbook request")?;
    let res = res
        .text()
        .await
        .chain_err(|| "Could not extract text body from addressbook response")?;
    println!("{:#?}", res);

    Ok(())
}

pub async fn addressbook_path(config: &Config, client: &Client) -> Result<String> {
    let path = String::from("/");
    let path = fetch_current_user_principal_url(&config, &client, path).await?;
    let path = fetch_addressbook_home_set_url(&config, &client, path).await?;
    let path = fetch_addressbook_url(&config, &client, path).await?;

    Ok(path)
}
