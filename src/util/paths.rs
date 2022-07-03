use crate::util::error::STError;
use crate::util::log::log;

use std::fs::File;
use std::io::Write;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

fn touch(path: &Path) -> io::Result<()> {
    match fs::OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn mkdir(dir: String) -> Result<PathBuf, STError> {
    let dir = shellexpand::full(&dir)?.to_string();
    let dir = Path::new(&dir);

    fs::create_dir_all(dir)?;
    Ok(dir.to_path_buf())
}

pub fn cache_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_TUI_CACHE_DIR") {
        Ok(dir) => dir,
        _ => "~/.cache/steam-tui".to_string(),
    };
    mkdir(dir)
}

pub fn config_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_TUI_DIR") {
        Ok(dir) => dir,
        _ => "~/.config/steam-tui".to_string(),
    };
    mkdir(dir)
}

pub fn script_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_TUI_SCRIPT_DIR") {
        Ok(dir) => dir,
        _ => format!("{}/scripts", cache_directory()?.as_path().display()),
    };
    mkdir(dir)
}

pub fn icon_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_TUI_ICON_DIR") {
        Ok(dir) => dir,
        _ => format!("{}/icons", cache_directory()?.as_path().display()),
    };
    mkdir(dir)
}

pub fn steam_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_APP_DIR") {
        Ok(dir) => dir,
        _ => "~/.steam/steam/Steamapps/common/".to_string(),
    };
    mkdir(dir)
}

pub fn steam_run_wrapper() -> Result<PathBuf, STError> {
    let run = match env::var("STEAM_RUN_WRAPPER") {
        Ok(run) => run,
        _ => "~/.steam/bin32/steam-runtime/run.sh".to_string(),
    };
    let run = shellexpand::full(&run)?.to_string();
    let run = Path::new(&run);

    if run.exists() {
        Ok(run.to_path_buf())
    } else {
        Err(STError::Problem(format!(
            "Run wrapper doesn't exist: {:?}",
            run
        )))
    }
}

pub fn executable_join(executable: &str, installdir: &str) -> Result<PathBuf, STError> {
    let installdir = Path::new(installdir);
    let executable = Path::new(executable);
    let script_path = installdir.join(executable);
    Ok(script_path)
}

pub fn icon_exists(id: i32) -> Result<PathBuf, STError> {
    let dir = icon_directory()?;
    let icon = &format!("{}.ico", id);
    let icon = Path::new(icon);
    let icon = dir.join(icon);
    if icon.exists() {
        Ok(icon)
    } else {
        Err(STError::Problem(format!("Icon doesn't exist: {:?}", icon)))
    }
}

pub fn icon_save(id: i32, icon: &[u8]) -> Result<(), STError> {
    let dir = icon_directory()?;
    let icon_path = &format!("{}.ico", id);
    let icon_path = Path::new(icon_path);
    let icon_path = dir.join(icon_path);
    let mut file = File::create(icon_path)?;
    file.write_all(icon)?;
    Ok(())
}

pub fn executable_exists(executable: &str) -> Result<PathBuf, STError> {
    let dir = steam_directory()?;
    let executable = Path::new(executable);
    let script_path = dir.join(executable);
    log!(script_path);
    if script_path.exists() {
        Ok(script_path)
    } else {
        Err(STError::Problem("Executable doesn't exist".to_string()))
    }
}

fn script_location(file: &Path, contents: &str) -> Result<PathBuf, STError> {
    let dir = script_directory()?;
    let script_path = dir.join(file);
    let mut f = fs::File::create(&script_path)?;
    f.write_all(contents.as_bytes())?;
    Ok(script_path)
}

pub fn install_script_location(login: String, id: i32) -> Result<PathBuf, STError> {
    let file = &format!("{}.install", id);
    let file = Path::new(file);
    let contents = format!(
        r#"
login {}
app_update "{}" -validate
quit
"#,
        login, id
    );
    script_location(file, &contents)
}

pub fn launch_script_location(login: String, id: i32) -> Result<PathBuf, STError> {
    let file = &format!("{}.launch", id);
    let file = Path::new(file);
    let contents = format!(
        r#"
login {}
app_update "{}" -validate
app_run {}
quit
"#,
        login, id, id
    );
    script_location(file, &contents)
}

pub fn config_location() -> Result<PathBuf, STError> {
    let dir = config_directory()?;
    let config_path = Path::new("config.json");
    let config_path = dir.join(config_path);
    touch(&config_path)?;
    Ok(config_path)
}

pub fn cache_location() -> Result<PathBuf, STError> {
    let dir = cache_directory()?;
    let cache_path = Path::new("games.json");
    let cache_path = dir.join(cache_path);
    touch(&cache_path)?;
    Ok(cache_path)
}

pub fn invalidate_cache() -> Result<(), STError> {
    fs::remove_file(cache_location()?)?;
    Ok(())
}
