use crate::interface::*;

use crate::util::{
    error::STError,
    log::log,
    parser::*,
    paths::{
        cache_location, executable_exists, install_script_location, launch_script_location,
        steam_run_wrapper,
    },
};

use port_scanner::scan_port;

use std::process;
use std::sync::Arc;

use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;

const STEAM_PORT: u16 = 57343;

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

    // TODO(#20) Pass in Arcs for download status into threads. If requested state is in
    // downloading, show satus.
    let mut downloading: HashSet<i32> = HashSet::new();

    // Cleanup the steam process if steam-tui quits.
    let mut cleanup: Option<Sender<bool>> = None;

    loop {
        queue.push_front(receiver.recv()?);
        loop {
            match queue.pop_back() {
                None => break,
                Some(Command::StartClient) => {
                    if !scan_port(STEAM_PORT) {
                        let (sender, termination) = channel();
                        cleanup = Some(sender);
                        thread::spawn(move || {
                            let mut child = process::Command::new("steam")
                                .args(vec![
                                    "-console",
                                    "-dev",
                                    "-nofriendsui",
                                    "-no-browser",
                                    "+open",
                                    "steam://",
                                ])
                                .stdout(process::Stdio::null())
                                .stderr(process::Stdio::null())
                                .spawn()
                                .unwrap();

                            // TODO: Currently doesn't kill all grand-children processes.
                            while let Ok(terminate) = termination.recv() {
                                if terminate {
                                    let _ = child.kill();
                                    break;
                                }
                            }
                        });
                    }
                }
                Some(Command::Restart) => {
                    let mut state = state.lock()?;
                    *state = State::LoggedOut;
                    cmd = SteamCmd::new()?;
                    let user = match account {
                        Some(ref acct) => acct.account.clone(),
                        _ => "".to_string(),
                    };
                    queue.push_front(Command::Cli(format!("login {}", user)));
                }
                Some(Command::Install(id)) => {
                    if let Some(ref acct) = account {
                        if downloading.contains(&id) {
                            continue;
                        }
                        downloading.insert(id);
                        let name = acct.account.clone();
                        thread::spawn(move || {
                            if let Err(err) = SteamCmd::script(
                                install_script_location(name.clone(), id)
                                    .unwrap()
                                    .to_str()
                                    .expect("Installation thread failed."),
                            ) {
                                let err = format!("{:?}", err);
                                log!("Install script for:", name, "failed", err);
                            }
                        });
                    };
                }
                Some(Command::Run(id, launchables)) => {
                    // IF steam is running (we can check for port tcp/57343), then
                    //   SteamCmd::script("login, app_run <>, quit")
                    // otherwise attempt to launch normally.
                    if scan_port(STEAM_PORT) {
                        if let Some(ref acct) = account {
                            let name = acct.account.clone();
                            thread::spawn(move || {
                                if let Err(err) = SteamCmd::script(
                                    launch_script_location(name.clone(), id)
                                        .unwrap()
                                        .to_str()
                                        .expect("Launch thread failed."),
                                ) {
                                    let err = format!("{:?}", err);
                                    log!("Run script for:", name, "failed", err);
                                }
                            });
                            break;
                        }
                    }
                    for launchable in launchables {
                        if let Ok(path) = executable_exists(&launchable.executable) {
                            let mut command = match launchable.platform {
                                Platform::Windows => vec![
                                    "wine".to_string(),
                                    path.into_os_string().into_string().unwrap(),
                                ],
                                _ => vec![path.to_str().unwrap_or("").to_string()],
                            };
                            let mut args = launchable
                                .arguments
                                .clone()
                                .split(' ')
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>();
                            command.append(&mut args);
                            let entry = match steam_run_wrapper() {
                                Ok(wrapper) => wrapper.into_os_string().into_string().unwrap(),
                                Err(STError::Problem(_)) => command.remove(0),
                                Err(err) => return Err(err), // unwrap and rewrap to explicitly note this is an err.
                            };
                            thread::spawn(move || {
                                let output =
                                    process::Command::new(entry).args(command).output().unwrap();
                                log!("Launching stdout:", &output.stdout);
                                log!("Launching stderr:", &output.stderr);
                            });
                            break;
                        }
                    }
                }
                // Execute and handles response to various SteamCmd Commands.
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
                                queue.push_front(Command::Cli("info".to_string()));
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
                                .map(|(_, l)| match *LICENSE_LEX.tokenize(l).as_slice() {
                                    ["packageID", id] => id.parse::<i32>().unwrap_or(-1),
                                    _ => -1,
                                })
                                .filter(|x| x >= &0)
                                .collect::<Vec<i32>>();
                            let total = keys.len();
                            updated += total as i32;
                            for key in keys {
                                queue.push_front(Command::Cli(format!(
                                    "package_info_print {}",
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
                                                        "app_info_print {}",
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
                            if let Ok(game) = Game::new(key, &mut lines) {
                                games.push(game);
                            }
                        }
                        ["app_status", _id] => {
                            sender.send(response.to_string())?;
                        }
                        ["quit"] => {
                            if let Some(cleanup) = cleanup {
                                let _ = cleanup.send(true);
                            }
                            sender.send(response.to_string())?;
                            return Ok(());
                        }
                        _ => {
                            // Send back response for debugging reasons.
                            sender.send(response.to_string())?;
                            // Fail since unknown commands should never be executed.
                            return Err(STError::Problem(format!(
                                "Unknown command sent {}",
                                response
                            )));
                        }
                    }

                    // If in Loading state, update progress.
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

/// Manages and interfaces with SteamCmd threads.
pub struct Client {
    receiver: Mutex<Receiver<String>>,
    sender: Mutex<Sender<Command>>,
    state: Arc<Mutex<State>>,
}

impl Client {
    /// Spawns a StemCmd process to interface with.
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

    /// Ensures `State` is `State::LoggedIn`.
    pub fn is_logged_in(&self) -> Result<bool, STError> {
        Ok(self.get_state()? == State::LoggedIn)
    }

    pub fn get_state(&self) -> Result<State, STError> {
        Ok(self.state.lock()?.clone())
    }

    /// Runs installation script for the provided game id.
    pub fn install(&self, id: i32) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Install(id))?;
        Ok(())
    }

    /// Quits previous SteamCmd instance, and spawns a new one. This can be useful for getting more
    /// state data. Old processes fail to update due to short comings in SteamCmd.
    pub fn restart(&self) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Restart)?;
        Ok(())
    }

    /// Launches the provided game id using 'app_run' in steemcmd, or the raw executable depending
    /// on the Steam client state.
    pub fn run(&self, id: i32, launchables: &[Launch]) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Run(id, launchables.to_owned().to_vec()))?;
        Ok(())
    }

    /// Attempts to login the provided user string.
    pub fn login(&self, user: &str) -> Result<(), STError> {
        if user.is_empty() {
            return Err(STError::Problem(
                "Blank string. Requires user to log in.".to_string(),
            ));
        }
        let mut state = self.state.lock()?;
        *state = State::LoggedOut;
        let sender = self.sender.lock()?;
        sender.send(Command::Cli(format!("login {}", user)))?;
        Ok(())
    }

    /// Starts off the process of parsing all games from SteamCmd. First `State` is set to be in an
    /// unloaded state for `State::Loaded`.  The process start by calling 'licenses_print' which
    /// then extracts packageIDs, and calls 'package_info_print' for each package. This in turn
    /// extracts appIDs, and gets app particular data by calling 'app_info_print' and binds it to a
    /// `Game` object. When all data is loaded, the games are dumped to a file and the state is
    /// changed to `State::LoggedIn` indicating that all data has been extracted and can be
    /// presented.
    /// TODO(#8): Check for cached games prior to reloading everything, unless explicitly
    /// restarted.
    pub fn load_games(&self) -> Result<(), STError> {
        let mut state = self.state.lock()?;
        *state = State::Loaded(0, -1);
        let sender = self.sender.lock()?;
        sender.send(Command::Cli(String::from("licenses_print")))?;
        Ok(())
    }

    /// Extracts games from cached location.
    pub fn games(&self) -> Result<Vec<Game>, STError> {
        let db_content = fs::read_to_string(cache_location()?)?;
        let parsed: Vec<Game> = serde_json::from_str(&db_content)?;
        Ok(parsed)
    }

    /// Binds data from 'app_status' to a `GameStatus` object.
    pub fn status(&self, id: i32) -> Result<GameStatus, STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Cli(format!("app_status {}", id)))?;
        let receiver = self.receiver.lock()?;
        GameStatus::new(&receiver.recv()?)
    }

    /// Started up a headless steam instance in the background so that games can be launched
    /// through steamcmd.
    pub fn start_client(&self) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::StartClient)?;
        Ok(())
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
        let _ = sender.send(Command::Cli(String::from("quit")));
        let receiver = self.receiver.lock().expect("In destructor");
        let _ = receiver.recv();
    }
}
#[cfg(test)]
mod tests {
    use crate::client::{Client, State};
    use crate::util::error::STError;
    use crate::util::parser::Command;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn test_polluted_data() {
        let (tx1, receiver) = channel();
        let (sender, rx2) = channel();
        Client::start_process(Arc::new(Mutex::new(State::LoggedOut)), tx1, rx2);
        let pollution = String::from("pollution ™️ ö ®Ø 天 🎉 Maxisâ¢\n\n\n\nquit\nbash");
        sender
            .send(Command::Cli(pollution.clone()))
            .expect("Fails to send message...");
        assert!(&receiver
            .recv()
            .expect("Channel dies")
            .contains(&"pollution".to_string()));
    }

    #[test]
    fn test_implicit_line_ending() {
        let (tx1, receiver) = channel();
        let (sender, rx2) = channel();
        Client::start_process(Arc::new(Mutex::new(State::LoggedOut)), tx1, rx2);
        let message = String::from("doesn't hang");
        sender
            .send(Command::Cli(message.clone()))
            .expect("Fails to send message...");
        assert!(&receiver
            .recv()
            .expect("Channel dies")
            .contains(&"Command not found: doesn't".to_string()));
    }

    #[test]
    fn test_blank_login() {
        let client = Client::new();
        let result = client.login("");
        if let Err(STError::Problem(expected)) = result {
            assert!(expected.contains(&"Blank".to_string()));
            return;
        }
        panic!("Failed to unwrap")
    }
}
