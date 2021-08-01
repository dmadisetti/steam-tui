use crate::util::error::STError;

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

fn mkdir(dir:String) -> Result<PathBuf, STError> {
    let dir = shellexpand::full(&dir)?.to_string();
    let dir = Path::new(&dir);

    fs::create_dir_all(dir)?;
    Ok(dir.to_path_buf())
}

pub fn cache_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_TUI_DIR") {
        Ok(dir) => dir,
        _ => "~/.config/steam-tui".to_string(),
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

pub fn steam_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_APP_DIR") {
        Ok(dir) => dir,
        _ => "~/.steam/steam/steamapps/common/".to_string(),
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

pub fn image_exists(id: i32) -> Result<PathBuf, STError> {
    let dir = icon_location()?;
    let image = &format!("{}.ico", id);
    let image = Path::new(image);
    let image = dir.join(image);
    if image.exists() {
        Ok(image)
    } else {
        Err(STError::Problem(format!(
            "Image doesn't exist: {:?}",
            image
        )))
    }
}

pub fn executable_exists(executable: &str) -> Result<PathBuf, STError> {
    let dir = steam_directory()?;
    let executable = Path::new(executable);
    let script_path = dir.join(executable);
    if script_path.exists() {
        Ok(script_path)
    } else {
        Err(STError::Problem("Executable doesn't exist".to_string()))
    }
}

fn script_location(file: &Path, contents: &str) -> Result<PathBuf, STError> {
    let dir = config_directory()?;
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

pub fn icon_location() -> Result<PathBuf, STError> {
    let dir = cache_directory()?;
    let icon_path = Path::new("icons/");
    let icon_path = dir.join(icon_path);
    fs::create_dir_all(&icon_path)?;
    Ok(icon_path)
}

pub fn cache_location() -> Result<PathBuf, STError> {
    let dir = cache_directory()?;
    let cache_path = Path::new("games.json");
    let cache_path = dir.join(cache_path);
    touch(&cache_path)?;
    Ok(cache_path)
}
