use crate::util::{error::STError, parser::*, stateful::Named};
use crate::interface::executable::Executable;

use serde::{Deserialize, Serialize};

const STEAM_CDN: &str = "https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/";

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
