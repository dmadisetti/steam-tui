use std::thread;

use crate::util::paths::{icon_exists, icon_save};

use crate::interface::Game;

pub fn update_img(
    selected: &Option<&Game>,
) -> Option<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
    if let Some(game) = selected {
        if let Ok(path) = icon_exists(game.id) {
            if let Some(path) = path.to_str() {
                if let Ok(data) = image::open(path) {
                    return Some(data.to_rgba());
                }
            }
        } else if let Some(url) = &game.icon_url {
            if let Ok(payload) = reqwest::blocking::get(url) {
                if let Ok(bytes) = payload.bytes() {
                    // cache here?
                    let id = game.id;
                    let to_save = bytes.clone();
                    thread::spawn(move || {
                        let _ = icon_save(id, &to_save);
                    });
                    if let Ok(data) = image::load_from_memory(&bytes) {
                        return Some(data.to_rgba());
                    }
                }
            }
        }
    }
    None
}
