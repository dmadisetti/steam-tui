use crate::interface::executable::Executable;
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
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub id: i32,
    pub name: String,
    pub developer: String,
    pub homepage: String,
    pub publisher: String,
    pub executable: Vec<Executable>,
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

    fn is_valid(&self) -> bool {
        // Slow, and a hack- but whatever.
        let config = Config::new().unwrap();
        !&config.hidden_games.contains(&self.id) && config.allowed_games.contains(&self.game_type)
    }
}
