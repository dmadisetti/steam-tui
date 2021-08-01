use crate::util::{error::STError, parser::*, paths::executable_join, stateful::Named};

use std::cmp::Ordering;
use std::process;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};

use serde::{Deserialize, Serialize};

const STEAM_CDN: &str = "https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/";

pub struct GameStatus {
    pub state: String,
    pub installdir: String,
    pub size: f64,
}

impl GameStatus {
    pub fn new(data: &str) -> Result<GameStatus, STError> {
        let data = data.lines();
        let data = data
            .map(|l| match *STATUS_LEX.tokenize(&l).as_slice() {
                ["state", state] => Some(state),
                ["dir", dir] => Some(dir),
                ["disk", disk] => Some(disk),
                _ => None,
            })
            .flatten()
            .collect::<Vec<&str>>();
        Ok(GameStatus {
            state: data.get(0).unwrap_or(&"").to_string(),
            installdir: data.get(1).unwrap_or(&"").to_string(),
            size: data
                .get(2)
                .unwrap_or(&"")
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.),
        })
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum Platform {
    Linux,
    Mac,
    Windows,
    Unknown,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Launch {
    pub platform: Platform,
    pub executable: String,
    pub arguments: String,
}
impl Launch {
    pub fn new(config: &HashMap<String, Datum>, installdir: &str) -> Result<Launch, STError> {
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
        Ok(Launch {
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
    // launches sorted by
    pub fn get_launches(
        config: &Option<Datum>,
        installdir: String,
    ) -> Result<Vec<Launch>, STError> {
        let mut launches = vec![];
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
                    launches.push(Launch::new(&config, &installdir)?);
                }
            }
            launches.sort_by(|a, b| match (&a.platform, &b.platform) {
                (&Platform::Linux, _) => Ordering::Less,
                (_, &Platform::Linux) => Ordering::Greater,
                (&Platform::Windows, _) => Ordering::Less,
                (_, &Platform::Windows) => Ordering::Greater,
                _ => Ordering::Equal,
            });
        }
        Ok(launches)
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum GameType {
    Game,
    Driver,
    Unknown,
}
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub id: i32,
    pub name: String,
    pub developer: String,
    pub homepage: String,
    pub publisher: String,
    pub launch: Vec<Launch>,
    pub game_type: GameType,
    pub icon_url: Option<String>,
}
impl Game {
    pub fn new(key: &str, lines: &mut std::str::Lines) -> Result<Game, STError> {
        let blank: Datum = Datum::Value("-".to_string());
        if let Datum::Nest(map) = parse(lines) {
            if let Some(map) = map.get(key) {
                let map = map.maybe_nest()?;
                if let (
                    Some(Datum::Nest(common)),
                    Some(Datum::Nest(extended)),
                    Some(Datum::Nest(config)),
                ) = (map.get("common"), map.get("extended"), map.get("config"))
                {
                    let game = Game {
                        id: key.parse::<i32>().unwrap_or(0),
                        name: common
                            .get("name")
                            .unwrap_or(&Datum::Value("<no name>".to_string()))
                            .maybe_value()?,
                        developer: extended.get("developer").unwrap_or(&blank).maybe_value()?,
                        homepage: extended.get("homepage").unwrap_or(&blank).maybe_value()?,
                        publisher: extended.get("publisher").unwrap_or(&blank).maybe_value()?,
                        launch: Launch::get_launches(
                            &config.get("launch").cloned(),
                            config.get("installdir").unwrap_or(&blank).maybe_value()?,
                        )?,
                        game_type: match common.get("driverversion") {
                            Some(Datum::Value(_)) => GameType::Driver,
                            _ => match common.get("type") {
                                Some(Datum::Value(value)) => match value.to_lowercase().as_str() {
                                    "game" => GameType::Game,
                                    _ => GameType::Unknown,
                                },
                                _ => GameType::Unknown,
                            },
                        },
                        icon_url: match common.get("clienticon") {
                            Some(Datum::Value(hash)) => {
                                Some(format!("{}/{}/{}.ico", STEAM_CDN, key, hash))
                            }
                            _ => None,
                        },
                    };
                    return Ok(game);
                }
            }
        }
        Err(STError::Problem("Could not extract game.".to_string()))
    }
}

impl Named for Game {
    fn get_name(&self) -> String {
        self.name.clone()
    }
    // Should be tuneable through config, but w/e
    fn is_valid(&self) -> bool {
        self.game_type == GameType::Game
    }
}

pub struct Account {
    pub account: String,
    _id: String,
    _language: String,
}
impl Account {
    pub fn new(data: &str) -> Result<Account, STError> {
        let data = data.lines();
        let data = data
            .map(|l| match *ACCOUNT_LEX.tokenize(&l).as_slice() {
                ["Account", account] => Some(account),
                ["SteamID", id] => Some(id),
                ["Language", lang] => Some(lang),
                _ => None,
            })
            .filter(|d| d.is_some())
            .collect::<Vec<Option<&str>>>();
        if data.len() != 3 {
            return Err(STError::Problem(
                "Account info response in unexpected format.".to_string(),
            ));
        }
        Ok(Account {
            account: data[0].unwrap_or("").to_string(),
            _id: data[1].unwrap_or("").to_string(),
            _language: data[2].unwrap_or("").to_string(),
        })
    }
}

pub struct SteamCmd {
    iter: std::io::Split<BufReader<process::ChildStdout>>,
    stdin: process::ChildStdin,
}

impl SteamCmd {
    fn with_args(args: Vec<&str>) -> Result<SteamCmd, STError> {
        let attempt = process::Command::new("steamcmd")
            .args(args.as_slice())
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .spawn();

        if let Err(err) = attempt {
            return Err(STError::Process(err));
        }
        let child = attempt?;

        let f = BufReader::new(
            child
                .stdout
                .ok_or_else(|| STError::Problem("Failed to attach to stdout.".to_string()))?,
        );
        let mut iter = f.split(0x1b);
        let stdin = child
            .stdin
            .ok_or_else(|| STError::Problem("Failed to attach to stdin..".to_string()))?;

        // Send start up data I guess yeah?
        iter.next();
        iter.next();
        iter.next();
        iter.next();

        Ok(SteamCmd { iter, stdin })
    }

    pub fn new() -> Result<SteamCmd, STError> {
        SteamCmd::with_args(vec![
            "+@ShutdownOnFailedCommand 0",
            "+@NoPromptForPassword 1",
        ])
    }

    pub fn script(script: &str) -> Result<SteamCmd, STError> {
        SteamCmd::with_args(vec![
            "+@ShutdownOnFailedCommand 1",
            "+@NoPromptForPassword 1",
            "+@sStartupScript",
            &format!("runscript {}", script),
        ])
    }
    pub fn write(&mut self, line: &str) -> Result<(), STError> {
        // Strip line endings
        let line: String = line.chars().filter(|&c| !"\n\r".contains(c)).collect();
        let line = format!("{}\n", line);
        self.stdin.write_all(line.as_bytes())?;
        Ok(())
    }
    pub fn maybe_next(&mut self) -> Result<Vec<u8>, STError> {
        match self.next() {
            Some(Ok(result)) => Ok(result),
            _ => Err(STError::Problem("Unable to read from stdin".into())),
        }
    }
}

impl Iterator for SteamCmd {
    type Item = Result<Vec<u8>, std::io::Error>;
    fn next(&mut self) -> Option<Result<Vec<u8>, std::io::Error>> {
        self.iter.next()
    }
}

impl Drop for SteamCmd {
    fn drop(&mut self) {
        // Failure is fine, because stopping anyway.
        let _ = self.write(&String::from("quit\n"));
    }
}
