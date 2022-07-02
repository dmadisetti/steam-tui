use crate::util::{error::STError, parser::*};

pub struct Account {
    pub account: String,
    _id: String,
    _language: String,
}
impl Account {
    pub fn new(data: &str) -> Result<Account, STError> {
        let data = data.lines();
        let data = data
            .map(|l| match *ACCOUNT_LEX.tokenize(l).as_slice() {
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
