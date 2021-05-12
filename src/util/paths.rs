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

pub fn config_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_TUI_DIR") {
        Ok(dir) => dir,
        _ => "~/.config/steam-tui".to_string(),
    };
    let dir = shellexpand::full(&dir)?.to_string();
    let dir = Path::new(&dir);

    fs::create_dir_all(dir)?;
    Ok(dir.to_path_buf())
}

pub fn steam_directory() -> Result<PathBuf, STError> {
    let dir = match env::var("STEAM_APP_DIR") {
        Ok(dir) => dir,
        _ => "~/.steam/steam/steamapps/common/".to_string(),
    };
    let dir = shellexpand::full(&dir)?.to_string();
    let dir = Path::new(&dir);

    fs::create_dir_all(dir)?;
    Ok(dir.to_path_buf())
}

pub fn executable_join(executable: &str, installdir: &str) -> Result<PathBuf, STError> {
    let installdir = Path::new(installdir);
    let executable = Path::new(executable);
    let script_path = installdir.join(executable);
    Ok(script_path)
}

pub fn image_exists(id: i32) -> Result<PathBuf, STError> {
    let dir = config_directory()?;
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

pub fn install_script_location(login: String, id: i32) -> Result<PathBuf, STError> {
    let dir = config_directory()?;
    let script_path = &format!("{}.install", id);
    let script_path = Path::new(script_path);
    let script_path = dir.join(script_path);
    let mut f = fs::File::create(&script_path)?;
    let contents = format!(
        r#"
login {}
app_update "{}" -validate
quit
"#,
        login, id
    );
    f.write_all(contents.as_bytes())?;
    Ok(script_path)
}

pub fn config_location() -> Result<PathBuf, STError> {
    let dir = config_directory()?;
    let config_path = Path::new("config.json");
    let config_path = dir.join(config_path);
    touch(&config_path)?;
    Ok(config_path)
}

pub fn cache_location() -> Result<PathBuf, STError> {
    let dir = config_directory()?;
    let config_path = Path::new("games.json");
    let config_path = dir.join(config_path);
    touch(&config_path)?;
    Ok(config_path)
}
