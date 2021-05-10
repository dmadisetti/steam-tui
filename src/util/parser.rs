extern crate regex;

use crate::interface::Launch;
use crate::util::error::STError;

use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub struct Lexer {
    regex: Regex,
}

impl Lexer {
    pub fn new(regex: &str) -> Lexer {
        Lexer {
            regex: Regex::new(regex).expect("Regexes are predefined and tested. This is safe."),
        }
    }
    pub fn tokenize<'a>(&self, string: &'a str) -> Vec<&'a str> {
        let captures = self.regex.captures(string).map(|captures| {
            captures
                .iter() // All the captured groups
                .skip(1) // Skipping the complete match
                .flat_map(|c| c) // Ignoring all empty optional matches
                .map(|c| c.as_str()) // Grab the original strings
                .collect::<Vec<_>>() // Create a vector
        });
        captures.unwrap_or_else(|| Vec::new())
    }
}

lazy_static! {
    pub static ref INPUT_LEX: Lexer = Lexer::new(
        r#"(?x)
           (login)\s+(\w) |
           (info) |
           (quit) |
           (licenses_print) |
           (package_info_print)\s+(\d+) |
           (app_info_print)\s+(\d+) |
           (app_status)\s+(\d+)
           "#
    );
    pub static ref ACCOUNT_LEX: Lexer = Lexer::new(
        r#"(?x)
           \s*(Account):\s*([^\s]+)\s* |
           \s*(SteamID):\s*([^\s]+)\s* |
           \s*(Language):\s*([^\s]+)\s*
           "#,
    );
    pub static ref STATUS_LEX: Lexer = Lexer::new(
        r#"(?x)
           .*install\s+(state):\s+([^,]+).* |
           .*(dir):\s+"([^"]+)".* |
           .*(disk):\s+(\d+).* |
           "#,
    );
    pub static ref LICENSE_LEX: Lexer = Lexer::new(r".*(packageID)\s+(\d+).*");
    static ref DATA_LEX: Lexer = Lexer::new(
        r#"(?x)
           \s*"([^"]+)"\s+"([^"]*)"\s* |
           \s*"([^"]+)"\s*$ |
           \s*(})\s*$ |
           "#,
    );
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Datum {
    Value(String),
    Nest(HashMap<String, Datum>),
}

impl Datum {
    pub fn maybe_value(&self) -> Result<String, STError> {
        match self {
            Datum::Value(value) => Ok(value.clone()),
            _ => Err(STError::Problem("woops".to_string())),
        }
    }
    pub fn maybe_nest(&self) -> Result<HashMap<String, Datum>, STError> {
        match self {
            Datum::Nest(map) => Ok(map.clone()),
            _ => Err(STError::Problem("woops".to_string())),
        }
    }
}

pub enum Command {
    Cli(String),
    Install(i32),
    Run(Vec<Launch>),
    Restart,
}

pub fn parse(block: &mut dyn Iterator<Item = &str>) -> Datum {
    let mut map = HashMap::new();
    while let Some(line) = block.next() {
        match DATA_LEX.tokenize(&line).as_slice() {
            &["}"] => {
                break;
            }
            &[key, value] => {
                map.insert(key.to_string(), Datum::Value(value.to_string()));
            }
            &[key] => {
                block.next();
                map.insert(key.to_string(), parse(block));
            }
            _ => {}
        }
    }
    Datum::Nest(map)
}
