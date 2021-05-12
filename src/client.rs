use crate::interface::*;

use crate::util::{
    error::STError,
    parser::*,
    paths::{cache_location, executable_exists, install_script_location},
};

use std::process;
use std::sync::Arc;

use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;

#[derive(PartialEq, Clone)]
pub enum State {
    LoggedOut,
    LoggedIn,
    Failed,
    Terminated(String),
    Loaded(i32, i32),
}

fn execute(
    state: Arc<Mutex<State>>,
    sender: Sender<String>,
    receiver: Receiver<Command>,
) -> Result<(), STError> {
    let mut cmd = SteamCmd::new()?;
    let mut queue = VecDeque::new();
    let mut games = Vec::new();
    let mut account: Option<Account> = None;
    let mut downloading: HashSet<i32> = HashSet::new();

    loop {
        queue.push_front(receiver.recv()?);
        loop {
            match queue.pop_back() {
                None => break,
                Some(Command::Restart) => {
                    let mut state = state.lock()?;
                    *state = State::LoggedOut;
                    cmd = SteamCmd::new()?;
                    let user = match account {
                        Some(ref acct) => acct.account.clone(),
                        _ => "".to_string(),
                    };
                    queue.push_front(Command::Cli(format!("login {}\n", user).to_string()));
                }
                Some(Command::Install(id)) => {
                    if let Some(ref acct) = account {
                        if downloading.contains(&id) {
                            continue;
                        }
                        downloading.insert(id);
                        let name = acct.account.clone();
                        thread::spawn(move || {
                            SteamCmd::script(
                                install_script_location(name, id)
                                    .unwrap()
                                    .to_str()
                                    .expect("Installation thread failed."),
                            )
                            .unwrap();
                        });
                    };
                }
                Some(Command::Run(launchables)) => {
                    for launchable in launchables {
                        if let Ok(path) = executable_exists(&launchable.executable) {
                            let command = match launchable.platform {
                                Platform::Windows => format!("wine {:?}", path),
                                _ => path.to_str().unwrap_or("").to_string(),
                            };
                            let args = launchable.arguments.clone();
                            thread::spawn(move || {
                                process::Command::new(command)
                                    .args(args.split(' '))
                                    .stdout(process::Stdio::null())
                                    .spawn()
                                    .unwrap();
                            });
                            break;
                        }
                    }
                }
                Some(Command::Cli(line)) => {
                    cmd.write(&line)?;
                    let mut updated = 0;
                    let waiting = queue.len();
                    let buf = cmd.maybe_next()?;
                    let response = String::from_utf8_lossy(&buf);
                    match *INPUT_LEX.tokenize(&line).as_slice() {
                        ["login", _] => {
                            if response.to_string().contains("Login Failure") {
                                let mut state = state.lock()?;
                                *state = State::Failed;
                            } else {
                                queue.push_front(Command::Cli("info\n".to_string()));
                            }
                        }
                        ["info"] => {
                            account = match Account::new(&response.to_string()) {
                                Ok(acct) => Some(acct),
                                _ => None,
                            };
                            let mut state = state.lock()?;
                            *state = State::Loaded(0, -2);
                        }
                        ["licenses_print"] => {
                            // Extract licenses
                            games = Vec::new();
                            let licenses = response.to_string();
                            let keys = licenses
                                .lines()
                                .enumerate()
                                .filter(|(i, _)| i % 4 == 0)
                                .map(|(_, l)| match *LICENSE_LEX.tokenize(&l).as_slice() {
                                    ["packageID", id] => id.parse::<i32>().unwrap_or(-1),
                                    _ => -1,
                                })
                                .filter(|x| x >= &0)
                                .collect::<Vec<i32>>();
                            let total = keys.len();
                            updated += total as i32;
                            for key in keys {
                                queue.push_front(Command::Cli(format!(
                                    "package_info_print {}\n",
                                    key
                                )));
                            }
                            let mut state = state.lock()?;
                            *state = State::Loaded(0, total as i32);
                        }
                        ["package_info_print", key] => {
                            let mut lines = response.lines();
                            updated += 1;
                            if let Datum::Nest(map) = parse(&mut lines) {
                                if let Some(map) = map.get(key) {
                                    if let Some(Datum::Nest(apps)) = map.maybe_nest()?.get("appids")
                                    {
                                        for wrapper in apps.values() {
                                            if let Datum::Value(id) = wrapper {
                                                let key = id.parse::<i32>().unwrap_or(-1);
                                                if key >= 0 {
                                                    queue.push_front(Command::Cli(format!(
                                                        "app_info_print {}\n",
                                                        key
                                                    )));
                                                }
                                            }
                                        }
                                    }
                                }
                            };
                            let mut state = state.lock()?;
                            match *state {
                                State::Loaded(_, _) => {}
                                _ => *state = State::Loaded(updated, queue.len() as i32),
                            }
                        }
                        ["app_info_print", key] => {
                            let mut lines = response.lines();
                            updated += 1;
                            if let Ok(game) = Game::new(&key, &mut lines) {
                                games.push(game);
                            }
                        }
                        ["app_status", _id] => {
                            sender.send(response.to_string())?;
                        }
                        ["quit"] => return Ok(()),
                        _ => {
                            sender.send(response.to_string())?;
                        }
                    }

                    let mut state = state.lock()?;
                    if let State::Loaded(o, e) = *state {
                        updated += o;
                        let total = e + (queue.len() - waiting) as i32;
                        *state = if updated == total {
                            games.sort_by(|a, b| a.name.cmp(&b.name));
                            fs::write(cache_location()?, serde_json::to_string(&games)?)?;
                            games = Vec::new();
                            State::LoggedIn
                        } else {
                            State::Loaded(updated, total)
                        }
                    }
                    // Iterate to scrub past Steam> prompt
                    let _ = cmd.maybe_next()?;
                }
            }
        }
    }
}

pub struct Client {
    receiver: Mutex<Receiver<String>>,
    sender: Mutex<Sender<Command>>,
    state: Arc<Mutex<State>>,
}

impl Client {
    pub fn new() -> Client {
        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();

        let client = Client {
            receiver: Mutex::new(rx1),
            sender: Mutex::new(tx2),
            state: Arc::new(Mutex::new(State::LoggedOut)),
        };
        Client::start_process(client.state.clone(), tx1, rx2);
        client
    }

    pub fn is_logged_in(&self) -> Result<bool, STError> {
        Ok(self.get_state()? == State::LoggedIn)
    }

    pub fn get_state(&self) -> Result<State, STError> {
        Ok(self.state.lock()?.clone())
    }

    pub fn install(&self, id: i32) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Install(id))?;
        Ok(())
    }

    pub fn restart(&self) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Restart)?;
        Ok(())
    }

    pub fn run(&self, launchables: &[Launch]) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Run(launchables.to_owned().to_vec()))?;
        Ok(())
    }

    pub fn login(&self, user: &str) -> Result<(), STError> {
        let mut state = self.state.lock()?;
        *state = State::LoggedOut;
        let sender = self.sender.lock()?;
        sender.send(Command::Cli(format!("login {}\n", user)))?;
        Ok(())
    }

    pub fn load_games(&self) -> Result<(), STError> {
        let mut state = self.state.lock()?;
        *state = State::Loaded(0, -1);
        let sender = self.sender.lock()?;
        sender.send(Command::Cli(String::from("licenses_print\n")))?;
        Ok(())
    }

    pub fn games(&self) -> Result<Vec<Game>, STError> {
        let db_content = fs::read_to_string(cache_location()?)?;
        let parsed: Vec<Game> = serde_json::from_str(&db_content)?;
        Ok(parsed)
    }

    pub fn status(&self, id: i32) -> Result<GameStatus, STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Cli(format!("app_status {}\n", id)))?;
        let receiver = self.receiver.lock()?;
        GameStatus::new(&receiver.recv()?)
    }

    fn start_process(
        state: Arc<Mutex<State>>,
        sender: Sender<String>,
        receiver: Receiver<Command>,
    ) {
        thread::spawn(move || {
            let local = state.clone();
            match execute(state, sender, receiver) {
                Ok(_) => {}
                Err(e) => {
                    let mut state = local
                        .lock()
                        .expect("We need to inform the other thread that this broke.");
                    *state = State::Terminated(format!("Fatal Error in client thread:\n{}", e));
                }
            };
        });
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let sender = self
            .sender
            .lock()
            .expect("In destructor, error handling is meaningless");
        let _ = sender.send(Command::Cli(String::from("quit\n")));
    }
}
#[cfg(test)]
mod tests {
    use crate::client::{Client, State};
    use crate::util::parser::Command;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn test_polluted_data() {
        let (tx1, receiver) = channel();
        let (sender, rx2) = channel();
        Client::start_process(Arc::new(Mutex::new(State::LoggedOut)), tx1, rx2);
        let pollution = String::from("pollution ™️ ö ®Ø 天 🎉 Maxisâ¢\n");
        sender
            .send(Command::Cli(pollution.clone()))
            .expect("Fails to send message...");
        assert!(&receiver
            .recv()
            .expect("Channel dies")
            .contains(&"pollution".to_string()));
    }
}
