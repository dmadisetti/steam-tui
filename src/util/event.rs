use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use crate::util::log::log;

use crossterm::{
    event::{read, Event as CrossEvent, KeyEvent, KeyCode},
};

pub enum Event<I> {
    Input(I),
    Tick,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event<KeyCode>>,
    stop: Arc<AtomicBool>,
    debounce: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            tick_rate: Duration::from_millis(500),
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let debounce = Arc::new(AtomicBool::new(true));

        let _input_handle = {
            let tx = tx.clone();
            let stop = stop.clone();
            let debounce = debounce.clone();
            thread::spawn(move || {
                //matching the key
                loop {
                    if let CrossEvent::Key(KeyEvent {
                        code: kc, ..
                    }) = read().unwrap() {
                        if debounce.load(Ordering::Relaxed) {
                            if let Err(err) = tx.send(Event::Input(kc)) {
                                log!(err);
                                return;
                            }
                            debounce.store(false, Ordering::Relaxed);
                        }
                        if stop.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            })
        };
        let _tick_handle = {
            let stop = stop.clone();
            thread::spawn(move || loop {
                if tx.send(Event::Tick).is_err() {
                    break;
                }
                thread::sleep(config.tick_rate);
                if stop.load(Ordering::Relaxed) {
                    return;
                }
            })
        };
        Events { rx, stop, debounce }
    }

    pub fn release(&self) {
        self.debounce.store(true, Ordering::Relaxed);
    }

    pub fn next(&self) -> Result<Event<KeyCode>, mpsc::RecvError> {
        self.rx.recv()
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
