extern crate steam_tui;

use std::io;

use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, layout::Rect, Terminal};

use tui_image_rgba_updated::{ColorMode, Image};

use steam_tui::util::event::{Event, Events};
use steam_tui::util::image::{update_img};
use steam_tui::util::stateful::StatefulList;

use steam_tui::app::{App, Mode};
use steam_tui::client::{Client, State};
use steam_tui::config::Config;
use steam_tui::interface::Game;

fn entry() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut img: Option<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> = None;

    // Setup event handlers
    let mut config = Config::new()?;
    let mut app = App::new(&config);
    let events = Events::new();
    let client = Client::new();

    if !app.user.is_empty() {
        client.login(&app.user)?;
    }
    let mut game_list: StatefulList<Game> = StatefulList::new();
    game_list.restart();

    loop {
        terminal.draw(|frame| {
            let layout = App::build_layout();
            let placement = layout.split(frame.size());
            let help = match app.mode {
                Mode::Normal => App::build_help(),
                Mode::Terminated(_) => App::build_terminated_help(),
                Mode::Login | Mode::Failed => App::build_login(app.user.clone()),
                Mode::Loading => match client.get_state() {
                    Ok(State::Loaded(count, of)) => App::build_loaded(count, of),
                    _ => App::build_loading(),
                },
                Mode::Searching => App::build_query_searching(game_list.query.clone()),
                Mode::Searched => App::build_query(game_list.query.clone()),
            };
            match &app.mode {
                Mode::Failed => frame.render_widget(App::build_splash_err(), placement[0]),
                Mode::Terminated(err) => {
                    frame.render_widget(App::build_splash_terminated(err.clone()), placement[0])
                }
                Mode::Loading | Mode::Login => {
                    frame.render_widget(App::build_splash(), placement[0]);
                }
                _ => {
                    let selected: i32 = match game_list.selected() {
                        Some(game) => game.id as i32,
                        None => 0,
                    };
                    let status = match client.status(selected) {
                        Ok(status) => Some(status),
                        _ => None,
                    };
                    let game_layout = App::build_game_layout();
                    let image_layout = App::build_image_layout();

                    let (left, right) = App::render_games(&game_list, status);
                    let game_placement = game_layout.split(placement[0]);
                    let image_placement = image_layout.split(Rect {
                        x: game_placement[1].width + game_placement[1].x - 40,
                        y: game_placement[1].height + game_placement[1].y - 20,
                        width: 40,
                        height: 20,
                    });

                    frame.render_stateful_widget(left, game_placement[0], &mut game_list.state);
                    frame.render_widget(right, game_placement[1]);
                    if let Some(image) = img.clone() {
                        frame.render_widget(
                            Image::with_img(image).color_mode(ColorMode::Rgba),
                            image_placement[0],
                        )
                    }
                }
            }
            frame.render_widget(help, placement[1]);
        })?;

        if let Event::Input(input) = events.next()? {
            match app.mode {
                Mode::Terminated(_) => {
                    if let Key::Char('q') = input {
                        break;
                    }
                }
                Mode::Normal | Mode::Searched => match input {
                    Key::Char('l') => {
                        app.mode = Mode::Login;
                        game_list.restart();
                    }
                    Key::Char('q') => {
                        break;
                    }
                    Key::Char('r') => {
                        app.mode = Mode::Loading;
                        client.restart()?;
                    }
                    Key::Down | Key::Char('j') | Key::Char('s') => {
                        game_list.next();
                        img = update_img(&game_list.selected());
                    }
                    Key::Up | Key::Char('k') | Key::Char('w') => {
                        game_list.previous();
                        img = update_img(&game_list.selected());
                    }
                    Key::Char('/') => {
                        app.mode = Mode::Searching;
                        game_list.unselect();
                    }
                    Key::Char('\n') => {
                        if let Some(game) = game_list.selected() {
                            client.run(game.id, &game.launch)?;
                        }
                    }
                    Key::Char(' ') => {
                        client.start_client()?;
                    }
                    Key::Char('d') => {
                        if let Some(game) = game_list.selected() {
                            client.install(game.id as i32)?;
                        }
                    }
                    Key::Esc => {
                        app.mode = Mode::Normal;
                        game_list.query = "".to_string();
                    }
                    _ => {}
                },
                Mode::Login | Mode::Failed => match input {
                    Key::Esc => {
                        if client.is_logged_in()? {
                            if game_list.query.is_empty() {
                                app.mode = Mode::Normal;
                            } else {
                                app.mode = Mode::Searched;
                            }
                            app.user = config.default_user.clone();
                        } else {
                            break;
                        }
                    }
                    Key::Char('\n') => {
                        let mut user = app.user.clone();
                        user.retain(|c| !c.is_whitespace());
                        if !user.is_empty() {
                            app.mode = Mode::Loading;
                            config.default_user = user;
                            client.login(&app.user)?;
                        }
                    }
                    Key::Backspace => {
                        app.user.pop();
                    }
                    Key::Char(c) => {
                        app.user.push(c);
                    }
                    _ => {}
                },
                Mode::Searching => match input {
                    Key::Esc => {
                        app.mode = Mode::Normal;
                        game_list.query = "".to_string();
                        img = update_img(&game_list.selected());
                    }
                    Key::Char('\n') => {
                        app.mode = Mode::Searched;
                    }
                    Key::Backspace => {
                        game_list.query.pop();
                        game_list.restart();
                    }
                    Key::Char(c) => {
                        game_list.query.push(c);
                        game_list.restart();
                        img = update_img(&game_list.selected());
                    }
                    Key::Down => {
                        game_list.next();
                    }
                    Key::Up => {
                        game_list.previous();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if app.mode == Mode::Loading {
            match client.get_state()? {
                State::Loaded(_, -2) => {
                    client.load_games()?;
                }
                State::LoggedIn => {
                    config.save()?;
                    let query = game_list.query.clone();
                    if query.is_empty() {
                        app.mode = Mode::Normal;
                    } else {
                        app.mode = Mode::Searched;
                    }
                    game_list = StatefulList::with_items(client.games()?);
                    terminal.clear()?;
                }
                State::Failed => {
                    app.mode = Mode::Failed;
                }
                _ => {}
            }
        }
        if let State::Terminated(err) = client.get_state()? {
            app.mode = Mode::Terminated(err);
        }
    }
    Ok(())
}

fn main() {
    match entry() {
        Ok(()) => {}
        Err(err) => println!("{:?}", err),
    }
}
