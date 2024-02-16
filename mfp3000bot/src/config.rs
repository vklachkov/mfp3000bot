use anyhow::Context;
use bstr::BString;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub telegram: Telegram,
    pub devices: Devices,
    pub scanner: HashMap<String, HashMap<BString, BString>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Telegram {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Devices {
    pub printer: Option<String>,
    pub scanner: Option<String>,
}

impl Config {
    pub fn read_from<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let dpath = path.display();

        let raw = fs::read_to_string(&path).with_context(|| format!("reading file '{dpath}'"))?;
        let config = toml::from_str(&raw).with_context(|| format!("parsing file '{dpath}'"))?;

        Ok(config)
    }
}