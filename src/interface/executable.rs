use crate::util::{error::STError, log::log, parser::*, paths::executable_join};

use std::cmp::Ordering;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum Platform {
    Linux,
    Mac,
    Windows,
    Unknown,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Executable {
    pub platform: Platform,
    pub executable: String,
    pub arguments: String,
}
impl Executable {
    pub fn new(config: &HashMap<String, Datum>, installdir: &str) -> Result<Executable, STError> {
        let platform = match config.get("config") {
            Some(Datum::Nest(config)) => match config.get("oslist") {
                Some(Datum::Value(ref platform)) => match platform.as_str() {
                    "linux" => Platform::Linux,
                    "windows" => Platform::Windows,
                    "macos" => Platform::Mac,
                    _ => Platform::Unknown,
                },
                _ => Platform::Unknown,
            },
            _ => Platform::Unknown,
        };
        Ok(Executable {
            platform,
            executable: executable_join(
                &config
                    .get("executable")
                    .unwrap_or(&Datum::Value("".into()))
                    .maybe_value()?,
                installdir,
            )?
            .to_str()
            .unwrap_or("")
            .to_string(),
            arguments: config
                .get("arguments")
                .unwrap_or(&Datum::Value("".into()))
                .maybe_value()?,
        })
    }
    // executables sorted by platform preference
    pub fn get_executables(
        config: &Option<Datum>,
        installdir: String,
    ) -> Result<Vec<Executable>, STError> {
        let mut executables = vec![];
        log!(config);
        if let Some(Datum::Nest(config)) = config {
            let mut keys = config
                .keys()
                .map(|k| k.parse::<i32>().unwrap_or(-1))
                .filter(|k| k >= &0)
                .collect::<Vec<i32>>();
            // UNstable sort recommended by clippy for primatives
            keys.sort_unstable();
            for key in keys {
                if let Some(Datum::Nest(config)) = config.get(&format!("{}", key)) {
                    executables.push(Executable::new(config, &installdir)?);
                }
            }
            executables.sort_by(|a, b| match (&a.platform, &b.platform) {
                (&Platform::Linux, _) => Ordering::Less,
                (_, &Platform::Linux) => Ordering::Greater,
                (&Platform::Windows, _) => Ordering::Less,
                (_, &Platform::Windows) => Ordering::Greater,
                _ => Ordering::Equal,
            });
        }
        Ok(executables)
    }
}
