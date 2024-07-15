use anyhow::Context;
use bstr::BString;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub telegram: Telegram,

    pub devices: Devices,

    pub print: Print,
    pub scan: Scan,

    #[serde(default = "Default::default")]
    pub scanner: HashMap<String, HashMap<BString, BString>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Telegram {
    pub token: String,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Devices {
    pub printer: Option<String>,
    pub scanner: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Print {
    pub paper_size: Option<libcups::options::MediaFormat>,
    pub orientation: Option<libcups::options::Orientation>,
    pub sides: Option<libcups::options::Sides>,
    pub color_mode: Option<libcups::options::ColorMode>,
    pub quality: Option<libcups::options::PrintQuality>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Scan {
    pub preview_dpi: u16,

    pub page_dpi: u16,

    pub page_quality: u8,

    #[serde(default = "Default::default")]
    pub common_options: HashMap<BString, BString>,
}

impl Config {
    pub fn read_from<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let dpath = path.display();

        let raw = fs::read_to_string(path).with_context(|| format!("reading file '{dpath}'"))?;
        let config = toml::from_str(&raw).with_context(|| format!("parsing file '{dpath}'"))?;

        Ok(config)
    }
}
