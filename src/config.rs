use error_chain::error_chain;
use serde::Deserialize;
use std::{env, fs::File, io::Read, path::PathBuf, process::Command};
use toml;

error_chain! {}

pub fn run_cmd(cmd: &str) -> Result<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    }
    .chain_err(|| "Run command failed")?;

    Ok(String::from_utf8(output.stdout).chain_err(|| "Invalid utf8 output")?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub ssl: Option<bool>,
    pub login: String,
    pub passwd_cmd: String,
    pub sync_dir: PathBuf,
}

impl Config {
    fn path_from_xdg() -> Result<PathBuf> {
        let path =
            env::var("XDG_CONFIG_HOME").chain_err(|| "Cannot find `XDG_CONFIG_HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push("cardamom");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_xdg_alt() -> Result<PathBuf> {
        let path = env::var("HOME").chain_err(|| "Cannot find `HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push(".config");
        path.push("cardamom");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_home() -> Result<PathBuf> {
        let path = env::var("HOME").chain_err(|| "Cannot find `HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push(".cardamomrc");

        Ok(path)
    }

    pub fn from_file() -> Result<Self> {
        let mut file = File::open(
            Self::path_from_xdg()
                .or_else(|_| Self::path_from_xdg_alt())
                .or_else(|_| Self::path_from_home())
                .chain_err(|| "Cannot find config path")?,
        )
        .chain_err(|| "Cannot open config file")?;

        let mut content = vec![];
        file.read_to_end(&mut content)
            .chain_err(|| "Cannot read config file")?;

        Ok(toml::from_slice(&content).chain_err(|| "Cannot parse config file")?)
    }

    pub fn ssl(&self) -> bool {
        self.ssl.unwrap_or(true)
    }

    pub fn passwd(&self) -> Result<String> {
        let passwd = run_cmd(&self.passwd_cmd)?;
        let passwd = passwd.trim_end_matches("\n").to_owned();

        Ok(passwd)
    }

    pub fn url(&self, path: &str) -> String {
        let scheme = if self.ssl() { "https" } else { "http" };
        format!("{}://{}:{}{}", &scheme, &self.host, &self.port, &path)
    }
}
