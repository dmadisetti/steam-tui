use std::thread;

use std::sync::Arc;
use std::sync::Mutex;

use crate::interface::game_status::GameStatus;

use crate::interface::executable::Executable;
use crate::interface::proton_data;
use crate::util::{error::STError, parser::*, stateful::Named};

use crate::config::Config;
use crate::util::log::log;

use serde::{Deserialize, Serialize};

const STEAM_CDN: &str = "https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/";

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum GameType {
    Game,
    DLC,
    Driver,

    // Other types, default hidden
    Application,
    Config,
    Demo,
    Tool,
    Unknown,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub id: i32,
    pub name: String,
    pub developer: String,
    pub homepage: String,
    pub publisher: String,
    pub executable: Vec<Executable>,
    pub game_type: GameType,
    pub icon_url: Option<String>,
    #[serde(skip)]
    proton_tier: Arc<Mutex<Option<String>>>,
    #[serde(skip)]
    status: Arc<Mutex<Option<GameStatus>>>,
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
                        executable: Executable::get_executables(
                            &config.get("executable").cloned(),
                            config.get("installdir").unwrap_or(&blank).maybe_value()?,
                        )?,
                        game_type: match common.get("driverversion") {
                            Some(Datum::Value(_)) => GameType::Driver,
                            _ => match common.get("type") {
                                Some(Datum::Value(value)) => match value.to_lowercase().as_str() {
                                    "game" => GameType::Game,
                                    "dlc" => GameType::DLC,

                                    "application" => GameType::Application,
                                    "config" => GameType::Config,
                                    "demo" => GameType::Demo,
                                    "tool" => GameType::Tool,
                                    unknown => {
                                        log!("Unknown game type", unknown);
                                        GameType::Unknown
                                    }
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
                        proton_tier: Arc::new(Mutex::new(None)),
                        status: Arc::new(Mutex::new(None)),
                    };
                    return Ok(game);
                }
            }
        }
        Err(STError::Problem("Could not extract game.".to_string()))
    }

    pub fn query_proton(&self) {
        let guard = {
            let mut tier = self.proton_tier.lock().unwrap();
            if None == *tier {
                *tier = Some("-".to_string());
                true
            } else {
                false
            }
        };
        if guard {
            let reference = self.proton_tier.clone();
            let id = self.id;
            thread::spawn(move || {
                if let Some(response) = proton_data::ProtonData::get(id) {
                    let mut status = reference.lock().unwrap();
                    *status = Some(response.format());
                }
            });
        }
    }

    pub fn get_proton(&self) -> String {
        let status = self.proton_tier.lock().unwrap();
        (*status).clone().unwrap_or_else(||"-".to_string())
    }

    pub fn get_status(&self) -> Option<GameStatus> {
        let status = self.status.lock().unwrap();
        (*status).clone()
    }

    pub fn update_status(self, new_status: GameStatus) {
        let mut status = self.status.lock().unwrap();
        *status = Some(new_status);
    }

    pub fn move_with_status(game: Game, maybe_status: Option<GameStatus>) -> Game {
        Game {
            status: Arc::new(Mutex::new(maybe_status)),
            ..game
        }
    }
}

impl Named for Game {
    fn get_name(&self) -> String {
        // Slow, and a hack- but whatever.
        let config = Config::new().unwrap();
        if config.favorite_games.contains(&self.id) {
            format!("♡ {}", self.name.clone())
        } else {
            self.name.clone()
        }
    }

    fn is_valid(&self) -> bool {
        // Wooowww, I hate this. Could be worse though?
        let config = Config::new().unwrap();
        !&config.hidden_games.contains(&self.id) && config.allowed_games.contains(&self.game_type)
    }
}
