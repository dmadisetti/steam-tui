use crate::util::{error::STError, parser::*};

#[derive(PartialEq, Debug, Clone)]
pub struct GameStatus {
    pub state: String,
    pub installdir: String,
    pub size: f64,
}

impl GameStatus {
    pub fn new(data: &str) -> Result<GameStatus, STError> {
        let data = data.lines();
        let data = data
            .filter_map(|l| match *STATUS_LEX.tokenize(l).as_slice() {
                ["state", state] => Some(state),
                ["dir", dir] => Some(dir),
                ["disk", disk] => Some(disk),
                _ => None,
            })
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
