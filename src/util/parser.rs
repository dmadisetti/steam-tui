extern crate regex;

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
                .flatten() // Ignoring all empty optional matches
                .map(|c| c.as_str()) // Grab the original strings
                .collect::<Vec<_>>() // Create a vector
        });
        captures.unwrap_or_default()
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
    pub static ref INSTALL_LEX: Lexer = Lexer::new(
        r#"(?x)
           .*(Update).*\((\d+)\s/\s(\d+)\)$ |
           .*(ERROR)!\s+(.*)$ |
           .*(Success).*$ |
           "#,
    );
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

pub fn parse(block: &mut dyn Iterator<Item = &str>) -> Datum {
    let mut map = HashMap::new();
    while let Some(line) = block.next() {
        match *DATA_LEX.tokenize(line).as_slice() {
            ["}"] => {
                break;
            }
            [key, value] => {
                map.insert(key.to_string(), Datum::Value(value.to_string()));
            }
            [key] => {
                block.next();
                map.insert(key.to_string(), parse(block));
            }
            _ => {}
        }
    }
    Datum::Nest(map)
}

#[cfg(test)]
mod tests {
    use crate::util::parser::{parse, Datum, INSTALL_LEX};
    #[test]
    fn test_parse_data() {
        let mut block = r#"
"hmm"
{
    "vdl" "format"
    "is"
    {
      "silly"
      {
          but hopefully
          this is robust
      }
      "hmm" "™️ and at least one with an accented o, ö and a registered trademark symbol, ®"
      "otherØ 天 🎉" "Do you have any games with non-standard latin characters? Ü Ø 天 🎉 ?"
    }
}
            "#
        .lines();
        let map = parse(&mut block);
        let maybe_map = map.maybe_nest();
        assert!(maybe_map.is_ok());
        let map = maybe_map.unwrap();
        assert_eq!(map.len(), 1);
        let map = map.values().next().unwrap().maybe_nest().unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(
            Some(&Datum::Value("format".to_string())),
            map.get(&"vdl".to_string())
        );
        let map = map.get(&"is".to_string()).unwrap().maybe_nest().unwrap();
        assert_eq!(map.len(), 3);
        let inner = map
            .get(&"silly".to_string())
            .expect("failed to unwrap")
            .maybe_nest()
            .expect("Failed to properly parse");
        assert_eq!(inner.len(), 0);
        let complex = map
            .get(&"otherØ 天 🎉".to_string())
            .unwrap()
            .maybe_value()
            .unwrap();
        assert!(complex.contains(&"Ü".to_string()));
    }
    #[test]
    fn test_parse_update_basic() {
        let line = "\u{1b}[0m Update state (0x3) reconfiguring, progress: 0.00 (0 / 0)";
        match *INSTALL_LEX.tokenize(line).as_slice(){
            ["Update", "0", "0"] => {}
            _ => panic!("Matched {:?}", INSTALL_LEX.tokenize(line)),
        }
    }
    #[test]
    fn test_parse_update() {
        let line = "\u{1b}[0m Update state (0x5) verifying install, progress: 0.00 (445476 / 12780261578)";
        match *INSTALL_LEX.tokenize(line).as_slice() {
            ["Update", "445476", "12780261578"] => {}
            _ => panic!("Matched {:?}", INSTALL_LEX.tokenize(line)),
        }
    }
    #[test]
    fn test_parse_update_continue() {
        let line = " Update state (0x5) verifying install, progress: 99.20 (12677647126 / 12780261578)";
        match *INSTALL_LEX.tokenize(line).as_slice() {
            ["Update", "12677647126", "12780261578"] => {}
            _ => panic!("Matched {:?}", INSTALL_LEX.tokenize(line)),
        }
    }
    #[test]
    fn test_parse_update_fail() {
        let line = "\u{1b}[0mERROR! Failed to install app '874260' (Invalid platform)";
        match *INSTALL_LEX.tokenize(line).as_slice() {
            ["ERROR", "Failed to install app '874260' (Invalid platform)"] => {}
            _ => panic!("Matched {:?}", INSTALL_LEX.tokenize(line)),
        }
    }
    #[test]
    fn test_parse_update_sucess() {
        let line = "Success! App '620' fully installed.";
        match *INSTALL_LEX.tokenize(line).as_slice() {
            ["Success"] => {}
            _ => panic!("Matched {:?}", INSTALL_LEX.tokenize(line)),
        }
    }
}
