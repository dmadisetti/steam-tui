use crate::util::error::STError;
use crate::util::paths::config_location;

use crate::interface::game::GameType;

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_user: String,
    pub hidden_games: Vec<i32>,
    pub allowed_games: Vec<GameType>,
}

impl Config {
    pub fn new() -> Result<Config, STError> {
        match serde_json::from_str(&fs::read_to_string(config_location()?)?) {
            Ok(config) => Ok(config),
            _ => {
                let config = Config {
                    default_user: "".to_string(),
                    hidden_games: vec![],
                    allowed_games: vec![GameType::Game, GameType::DLC],
                };
                config.save()?;
                Ok(config)
            }
        }
    }

    pub fn save(&self) -> Result<(), STError> {
        Ok(fs::write(
            config_location()?,
            serde_json::to_string(&self)?,
        )?)
    }
}
