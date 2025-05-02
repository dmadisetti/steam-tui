use crate::interface::{
    account::Account, executable::*, game::Game, game_status::*, steam_cmd::SteamCmd,
};

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

pub enum Command {
    Cli(String),
    Install(i32, Arc<Mutex<Option<GameStatus>>>),
    Run(i32, Vec<Executable>, Arc<Mutex<Option<GameStatus>>>),
    StartClient,
    Restart,
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
                // For flag reference see:
                //   https://developer.valvesoftware.com/wiki/Command_line_options#Steam_.28Windows.29
                //   and https://gist.github.com/davispuh/6600880
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
                Some(Command::Install(id, status)) => {
                    if let Some(ref acct) = account {
                        if downloading.contains(&id) {
                            continue;
                        }
                        downloading.insert(id);
                        let name = acct.account.clone();
                        thread::spawn(move || {
                            {
                                let mut reference = status.lock().unwrap();
                                *reference = Some(GameStatus::msg(&*reference, "processing..."));
                            }
                            match SteamCmd::script(
                                install_script_location(name.clone(), id)
                                    .unwrap()
                                    .to_str()
                                    .expect("Installation thread failed."),
                            ) {
                                Ok(mut cmd) => {
                                    // Scrub past unused data.
                                    for _ in 1..15 {
                                        cmd.next();
                                    }
                                    while let Ok(buf) = cmd.maybe_next() {
                                        let response = String::from_utf8_lossy(&buf);
                                        // TODO: Investigate why download updates don't seem to
                                        // appear...
                                        match *INSTALL_LEX.tokenize(&response).as_slice() {
                                            ["Update", a, b] => {
                                                let a = a.parse::<f64>().unwrap_or(0.);
                                                let b = b.parse::<f64>().unwrap_or(1.);
                                                let mut reference = status.lock().unwrap();
                                                let update =
                                                    format!("downloading {}%", 100. * a / b);
                                                *reference =
                                                    Some(GameStatus::msg(&*reference, &update));
                                            }
                                            ["ERROR", msg] => {
                                                let mut reference = status.lock().unwrap();
                                                let update = format!("Failed: {}", msg);
                                                *reference =
                                                    Some(GameStatus::msg(&*reference, &update));
                                            }
                                            ["Success"] => {
                                                let mut reference = status.lock().unwrap();
                                                let size = match &*reference {
                                                    Some(gs) => gs.size,
                                                    _ => 0.,
                                                };
                                                *reference = Some(GameStatus {
                                                    state: "Success!".to_string(),
                                                    installdir: "".to_string(),
                                                    size,
                                                });
                                                // TODO: call app_status and update after success.
                                            }
                                            _ => {
                                                log!("unmatched", response);
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    let err = format!("{:?}", err);
                                    let mut reference = status.lock().unwrap();
                                    *reference = Some(GameStatus {
                                        state: format!("Failed: {}", err),
                                        installdir: "".to_string(),
                                        size: 0.,
                                    });
                                    log!("Install script for:", name, "failed", err);
                                }
                            }
                        });
                    };
                }
                Some(Command::Run(id, executables, status)) => {
                    {
                        let mut reference = status.lock().unwrap();
                        *reference = Some(GameStatus::msg(&*reference, "launching..."));
                    }
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
                                    {
                                        let mut reference = status.lock().unwrap();
                                        *reference = Some(GameStatus::msg(
                                            &*reference,
                                            &format!("Error with script (trying direct): {}", err),
                                        ));
                                    }
                                    // Try again as per #51
                                    run_process(
                                        "steam".to_string(),
                                        vec![
                                            "-silent".to_string(),
                                            "-applaunch".to_string(),
                                            id.to_string(),
                                        ],
                                        status,
                                    );
                                }
                            });
                            break;
                        }
                    }
                    let mut launched = false;
                    for launchable in executables {
                        if let Ok(path) = executable_exists(&launchable.executable) {
                            log!(path);
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
                            log!("Finding entry");
                            let entry = match steam_run_wrapper(id) {
                                Ok(wrapper) => wrapper.into_os_string().into_string().unwrap(),
                                Err(STError::Problem(_)) => command.remove(0),
                                Err(err) => {
                                    let mut reference = status.lock().unwrap();
                                    *reference = Some(GameStatus::msg(
                                        &*reference,
                                        "Could not find entry program.",
                                    ));
                                    return Err(err);
                                } // unwrap and rewrap to explicitly note this is an err.
                            };
                            log!("Exits loop");
                            let status = status.clone();
                            thread::spawn(move || {
                                {
                                    let mut reference = status.lock().unwrap();
                                    *reference = Some(GameStatus::msg(&*reference, "running..."));
                                }
                                run_process(entry, command, status);
                            });
                            launched = true;
                            break;
                        } else {
                            log!("Tried", launchable.executable);
                        }
                    }
                    if !launched {
                        let mut reference = status.lock().unwrap();
                        *reference = Some(GameStatus::msg(
                            &*reference,
                            "Failed: Could not find executable to launch. Try setting $STEAM_APP_DIR",
                        ));
                    }
                }
                // Execute and handles response to various SteamCmd Commands.
                Some(Command::Cli(line)) => {
                    cmd.write(&line)?;
                    let mut updated = 0;
                    let waiting = queue.len();
                    let buf = cmd.maybe_next()?;
                    let mut response = String::from_utf8_lossy(&buf);
                    match *INPUT_LEX.tokenize(&line).as_slice() {
                        ["login", _] => {
                            // BUG TEMP FIX: Scrub unhandled lines
                            while response == "[1m\nSteam>" || response == "[0m" {
                                if let Ok(buf) = cmd.maybe_next() {
                                    response = String::from_utf8_lossy(&buf).into_owned().into();
                                } else {
                                    cmd.write("")?;
                                }
                            }
                            let response = response.to_string();
                            if response.contains("Login Failure") || response.contains("FAILED") {
                                let mut state = state.lock()?;
                                *state = State::Failed;
                            } else {
                                queue.push_front(Command::Cli("info".to_string()));
                            }
                            log!("login");
                        }
                        ["info"] => {
                            account = match Account::new(&response.to_string()) {
                                Ok(acct) => Some(acct),
                                _ => None,
                            };
                            let mut state = state.lock()?;
                            *state = State::Loaded(0, -2);
                            log!("info");
                        }
                        ["licenses_print"] => {
                            // Extract licenses
                            if response == "[0m" {
                                continue;
                            }

                            games = Vec::new();
                            let licenses = response.to_string();
                            let keys = keys_from_licenses(licenses);
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
                            log!("licenses_print");
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
                            log!("package_info_print");
                        }
                        ["app_info_print", key] => {
                            updated += 1;
                            log!("Checking game");
                            // Bug requires additional scan
                            // do a proper check here in case this is ever fixed.
                            // A bit of a hack, but will do for now.
                            let mut response = response;
                            log!(response);
                            if response == "[0m" {
                                cmd.write("")?;
                                cmd.write("")?;
                                while !response.starts_with("[0mAppID") {
                                    if let Ok(buf) = cmd.maybe_next() {
                                        response =
                                            String::from_utf8_lossy(&buf).into_owned().into();
                                    }
                                }
                            }
                            let mut lines = response.lines();

                            match Game::new(key, &mut lines) {
                                Ok(game) => {
                                    log!("got game");
                                    games.push(game);
                                    log!(key);
                                }
                                Err(err) => {
                                    log!(err)
                                }
                            };
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
                    let buf = cmd.maybe_next()?;
                    let mut prompt = String::from_utf8_lossy(&buf);
                    log!(prompt);
                    while prompt != "[1m\nSteam>" {
                        if let Ok(buf) = cmd.maybe_next() {
                            prompt = String::from_utf8_lossy(&buf).into_owned().into();
                        } else {
                            cmd.write("")?;
                        }
                    }
                }
            }
        }
    }
}

fn run_process(entry: String, command: Vec<String>, status: Arc<Mutex<Option<GameStatus>>>) {
    match process::Command::new(entry).args(command).output() {
        Ok(output) => {
            let stderr = output.stderr.clone();
            let stderr_snippet = &(String::from_utf8_lossy(&stderr)[..50]);
            let mut reference = status.lock().unwrap();
            *reference = Some(GameStatus::msg(
                &*reference,
                &(match output.status.code() {
                    Some(0) => format!("ran (success)"),
                    Some(n) => format!("failed with code {}: ({}...)", n, stderr_snippet),
                    None => format!("Process terminated."),
                }),
            ));

            log!("Launching stdout:", &std::str::from_utf8(&output.stdout));
            log!("Launching stderr:", &std::str::from_utf8(&output.stderr));
        }
        Err(err) => {
            let mut reference = status.lock().unwrap();
            *reference = Some(GameStatus::msg(
                &*reference,
                &format!("failed to launch: {}", err),
            ));
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
    pub fn install(&self, game: &Game) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Install(game.id as i32, game.status_counter()))?;
        Ok(())
    }

    /// Quits previous SteamCmd instance, and spawns a new one. This can be useful for getting more
    /// state data. Old processes fail to update due to short comings in SteamCmd.
    pub fn restart(&self) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Restart)?;
        Ok(())
    }

    /// Launches the provided game id using 'app_run' in steamcmd, or the raw executable depending
    /// on the Steam client state.
    pub fn run(&self, game: &Game) -> Result<(), STError> {
        let sender = self.sender.lock()?;
        sender.send(Command::Run(
            game.id,
            game.executable.to_owned().to_vec(),
            game.status_counter(),
        ))?;
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
        let mut processed: Vec<Game> = parsed
            .iter()
            .map(|game| Game::move_with_status((*game).clone(), self.status(game.id).ok()))
            .collect();
        processed.dedup_by(|a, b| a.id == b.id);
        Ok(processed)
    }

    /// Binds data from 'app_status' to a `GameStatus` object.
    pub fn status(&self, id: i32) -> Result<GameStatus, STError> {
        log!("Getting status for", id);
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

// Just some helpers broken out for testing
fn keys_from_licenses(licenses: String) -> Vec<i32> {
    licenses
        .lines()
        .enumerate()
        .filter(|(i, _)| i % 4 == 0)
        .map(|(_, l)| match *LICENSE_LEX.tokenize(l).as_slice() {
            ["packageID", id] => id.parse::<i32>().unwrap_or(-1),
            _ => -1,
        })
        .filter(|x| x >= &0)
        .collect::<Vec<i32>>()
}

#[cfg(test)]
mod tests {
    use crate::client::{Client, Command, State};
    use crate::util::error::STError;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::sync::Mutex;

    // Impure cases call to `steamcmd` which requires FHS.
    #[test]
    fn test_polluted_data_impure() {
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
    fn test_implicit_line_ending_impure() {
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
