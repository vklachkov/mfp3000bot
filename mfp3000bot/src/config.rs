use anyhow::Context;
#[cfg(feature = "scan")]
use bstr::BString;
use serde::Deserialize;
#[cfg(feature = "scan")]
use std::collections::HashMap;
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub telegram: Telegram,

    pub devices: Devices,

    pub print: Print,

    #[cfg(feature = "scan")]
    pub scan: Scan,

    #[cfg(feature = "scan")]
    #[serde(default = "Default::default")]
    pub scanner: HashMap<String, HashMap<BString, BString>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Telegram {
    pub token: String,
    #[serde(default)]
    pub allowed_group: Option<TelegramGroup>,
    pub allowed_users: Vec<String>,
    pub language: String,
}

#[derive(Debug, Clone, Copy)]
pub struct TelegramGroup {
    pub chat_id: i64,
    pub thread_id: Option<i32>,
}

impl<'de> Deserialize<'de> for TelegramGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let s = <&'de str>::deserialize(deserializer)?;
        if !s.is_ascii() {
            return Err(D::Error::invalid_value(
                serde::de::Unexpected::Str(s),
                &"an ASCII string",
            ));
        }

        let invalid_format = |_| {
            D::Error::invalid_value(
                serde::de::Unexpected::Str(s),
                &"string in format `chat_id` or `chat_id/thread_id`",
            )
        };

        let Some(separator_idx) = s.find('/') else {
            let chat_id = i64::from_str_radix(s, 10).map_err(invalid_format)?;
            return Ok(Self {
                chat_id,
                thread_id: None,
            });
        };

        let raw_chat_id = &s[..separator_idx];
        let chat_id = i64::from_str_radix(raw_chat_id, 10).map_err(invalid_format)?;

        let raw_thread_id = &s[separator_idx + 1..];
        let thread_id = i32::from_str_radix(raw_thread_id, 10).map_err(invalid_format)?;

        Ok(TelegramGroup {
            chat_id,
            thread_id: Some(thread_id),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Devices {
    pub printer: Option<String>,
    #[cfg(feature = "scan")]
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

#[cfg(feature = "scan")]
#[derive(Debug, Clone, Deserialize)]
pub struct Scan {
    pub preview_dpi: u16,

    pub page_dpi: u16,

    pub page_quality: u8,

    #[serde(default)]
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
