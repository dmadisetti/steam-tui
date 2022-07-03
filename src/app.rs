extern crate pretty_bytes;

use crate::util::stateful::{Named, StatefulList};

use crate::config::Config;
use crate::interface::game::Game;

use pretty_bytes::converter::convert;

use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Cell, List, ListItem, Paragraph, Row, Table},
};

const SPLASH: &str = r#"
 . .................................................................................................
  . ................................................................................................
. . ........nnnnMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMnnnn......... .
 ...........nnnnMMMMMMMMMMMMMMMMMMMMMMWKOxoc:;,''..',,;:coxOKWMMWMMMWMMMMMMMMMMMMMMMnnnn........ . .
. . ........nnnnMMM Steam TUI MMMMWKxl;..                  ..;lxKNWWMMMMMMMMMMMMMMMMnnnn......... .
 ...........nnnnMMMMMMMMMMMMMMMWKd:.                            .:xKWMMMMMMMMMMMMMMMnnnn........ . .
. . ........nnnnMMMMMMMMMMMWMWOc.                                  'l0WMMMMMMMMMMMMMnnnn......... .
 ...........nnnnMMMMMMMMMMMW0c.                           ...        .c0WMMMMMMMMMMMnnnn........ . .
. . ........nnnnMMMMMMMMMMNd.                        .,lxk00Okxl;.     .dNWMMMMMMMMMnnnn......... .
 ...........nnnnMMMMMMMMWK:.                       .o0NWWX000XNWN0o.    .cKWMMMMMMMMnnnn........ . .
. . ........nnnnMMMMMMMM0,                      .:x0WW0o::ccc::o0WW0:.    ;0WWMMMMMMnnnn......... .
 -----------nnnnMMMMMMMK;                       :KWWNd':kXWWWXk:'dNWK:     :KMMMMMMMnnnn-------- - -
- - --------nnnnMMMMWWNl                       .kWWWk.cXWMMMMWWNc.kWWx.     lNWWMMMMnnnn--------- -
 -----------nnnnMMMMMWk.                       :KMWWd.dWMMWMWWMWd.dWMk.     .kMMMMMMnnnn-------- - -
- - --------nnnnMMMMMNl                      .lXWWWM0,,0WMWMWWW0,,0WWd.      oWMMMMMnnnn--------- -
 -----------nnnnMMMMMXc.                   .'kNWMMWWW0:,lxO0Oxc,:0WWO'       :XMMMMMnnnn-------- - -
- - --------nnnnMMMMMWX0xl:'..     ...... .:0WMMMMMMMMNOdlcccld0NWXd' .. ... ;KMMMMMnnnn--------- -
 -----------nnnnMMMMMWMMMMWX0xl;'........'dXWWMMMMMMMMWWWWWWWWWN0d;......... ;KMMMMMnnnn-------- - -
- - --------nnnnMMMMMMMMMMMMMMMWXOxl:coxxONWWMMMMMMMMMMMMNOdol:,.............lNMMMMMnnnn--------- -
 -----------nnnnMMMMMMMMMMMMMMWMMMMMWWWXkdllkXMWMWWWMWN0o;...................xWMMMMMnnnn-------- - -
- - --------nnnnMMMMMMWXXNWMMWWMMMMMMMMWWWKx;:0WWMWN0d:'....................:KMWMMMMnnnn--------- -
 -----------nnnnMMMMMMWO::lx0XNWWMMWMMMMWWWW0,cNXxoc'......................;OWMMMMMMnnnn-------- - -
- - --------nnnnMMMMMMMNx'..',coxKWWWMMWWWWM0;lXx.........................'xWMMWMMMMnnnn--------- -
 -----------nnnnMMMMMMMMNx,.'....:ON0k0XNNXk::00:.......'................;kNWMMMMMMMnnnn-------- - -
' ' ''''''''nnnnMMMMMMMMMWO:''''.':x0xdoddoox0k:''''.'''''''''''''''''.'cOWMMMMMMMMMnnnn````````` `
 '''''''''''nnnnMMMMMMMMMMWXd;'''''':oxkkkkxoc,''''''''''''''''''''''';dXWWMMMMMMMMMnnnn```````` ` `
' ' ''''''''nnnnMMMMMMMMMMMWWKd:,'',,,',,,''','''',,'''''',''',,,,',;dKNWMMMMMMMMMMMnnnn````````` `
 '''''''''''nnnnMMMMMMMMMMMMMMWKxl;,,,,,,,,,,,,,,,,,,,,,,,,,,,,,,;lxKWWWMMMMMMMMMMMMnnnn```````` ` `
' ' ''''''''nnnnMMMMMMMMMMMMMMMMWN0xoc;;;;;;;;;;;;;;;;;;;;,,,;cox0NWWMMMMMMMMMMMMMMMnnnn````````` `
 '''''''''''nnnnMMMMMMMMMMMMMMMMWMMWWX0kdoc:::;;;;;;;:::clodk0XWWWMMMMMMMMMMMMMMMMMMnnnn```````` ` `
' ' ''''''''nnnnMMMMMMMMMMMMMMMMMMMMWWWWWNK0OkxdddddxxkO0XNWWMMMMMMMMMMMMMMMMMMMMMMMnnnn````````` `
 '''''''''''nnnnMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMnnnn```````` ` `
' ' ''''''''nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn````````` `
 ' '''''''''''''''''''''''''''''''''''''''''''''''''```````````````````````````````````````````` ` `
 ' '''''''''''''''''''''''''''''''''''''''''''''''''````````````````````````````````````````````` `
"#;

pub struct App {
    pub mode: Mode,
    pub user: String,
    pub highlight: Color,
}

#[derive(PartialEq, Clone)]
pub enum Mode {
    Login,
    Loading,
    Normal,
    Searching,
    Searched,
    Failed,
    Terminated(String),
}

impl App {
    pub fn new(config: &Config) -> App {
        let user = config.default_user.clone();
        let highlight = config.highlight;
        App {
            mode: if user.is_empty() {
                Mode::Login
            } else {
                Mode::Loading
            },
            user,
            highlight,
        }
    }

    pub fn build_layout() -> Layout {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(3)].as_ref())
    }
    pub fn build_image_layout() -> Layout {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 1)].as_ref())
    }
    pub fn build_game_layout() -> Layout {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)].as_ref())
    }
    pub fn build_splash_terminated(err: String) -> Paragraph<'static> {
        App::build_infobox(
            "Oh dear...".to_string(),
            format!(
                "Something has crashed.. For more details please refer to the error below:\n\n{}",
                err
            ),
            Alignment::Left,
        )
    }

    pub fn build_splash_err() -> Paragraph<'static> {
        App::build_infobox(
            "steam-tui".to_string(),
            format!(
                "{}\n Uhoh. Could not find credentials. Have you logged in?",
                SPLASH
            ),
            Alignment::Center,
        )
    }

    pub fn build_splash() -> Paragraph<'static> {
        App::build_infobox(
            "steam-tui".to_string(),
            SPLASH.to_string(),
            Alignment::Center,
        )
    }

    pub fn build_patience() -> Paragraph<'static> {
        App::build_infobox(
            "Welcome".to_string(),
            "Checking cache (on load, you can press 'r' to invalidate cache)".to_string(),
            Alignment::Left,
        )
    }

    fn build_infobox(title: String, content: String, alignment: Alignment) -> Paragraph<'static> {
        Paragraph::new(content)
            .style(Style::default())
            .alignment(alignment)
            .block(
                Block::default()
                    .borders(Borders::all())
                    .style(Style::default().fg(Color::White))
                    .title(title)
                    .border_type(BorderType::Plain),
            )
    }
    pub fn build_query(query: String) -> Paragraph<'static> {
        App::build_infobox(
            "Searching... (press esc to stop)".to_string(),
            query,
            Alignment::Left,
        )
    }
    pub fn build_query_searching(query: String) -> Paragraph<'static> {
        App::build_infobox(
            "Searching... (press Esc to stop, Enter to commit)".to_string(),
            query,
            Alignment::Left,
        )
    }
    pub fn build_loaded(count: i32, of: i32) -> Paragraph<'static> {
        let p = {
            if of < 0 {
                "Calculating...".to_string()
            } else {
                let p = 100. * (count as f32) / (of as f32);
                format!("Loading %{:.1}", p)
            }
        };
        App::build_infobox("Please wait".to_string(), p, Alignment::Left)
    }
    pub fn build_loading() -> Paragraph<'static> {
        App::build_infobox(
            "Please wait".to_string(),
            "Logging in and updating...".to_string(),
            Alignment::Left,
        )
    }
    pub fn build_login(username: String) -> Paragraph<'static> {
        App::build_infobox(
            "Login (Enter to submit)".to_string(),
            username,
            Alignment::Left,
        )
    }
    pub fn build_help() -> Paragraph<'static> {
        App::build_infobox(
            "Help".to_string(),
            "[/] Search | [d]ownload  | [l]ogin | [Enter]xecute | Up (k, w) | Down (j, s) | [q]uit | [Space]team"
                .to_string(),
            Alignment::Left,
        )
    }
    pub fn build_terminated_help() -> Paragraph<'static> {
        App::build_infobox(
            "Woops.".to_string(),
            "Press q to quit.".to_string(),
            Alignment::Left,
        )
    }

    pub fn render_games<'a>(
        highlight: Color,
        game_list: &StatefulList<Game>,
    ) -> (List<'a>, Table<'a>) {
        let games = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Games")
            .border_type(BorderType::Plain);

        let items: Vec<_> = game_list
            .activated()
            .iter()
            .map(|game| {
                let fg = {
                    if let Some(status) = game.get_status() {
                        if status.state == "uninstalled" {
                            Color::DarkGray
                        } else {
                            Color::White
                        }
                    } else {
                        Color::DarkGray
                    }
                };
                ListItem::new(Spans::from(vec![Span::styled(
                    game.get_name(),
                    Style::default().fg(fg),
                )]))
            })
            .collect();

        let list = List::new(items).block(games).highlight_style(
            Style::default()
                .bg(highlight)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        let details = match game_list.selected() {
            Some(selected) => {
                let spacer = Row::new(vec![Cell::from(Span::raw(" "))]);
                // Construct table head (id, name)
                let mut table = vec![
                    Row::new(vec![
                        Cell::from(Span::styled(
                            "ID",
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                        Cell::from(Span::styled(
                            "Name",
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                    ]),
                    Row::new(vec![
                        Cell::from(Span::raw(selected.id.to_string())),
                        Cell::from(Span::raw(selected.get_name())),
                    ]),
                    spacer.clone(),
                ];
                // Construct table details
                for &(heading, value) in &[
                    ("Homepage", &selected.homepage),
                    ("Developer", &selected.developer),
                    ("Publisher", &selected.publisher),
                ] {
                    table.push(Row::new(vec![
                        Cell::from(Span::styled(
                            heading,
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                        Cell::from(Span::raw(value.clone())),
                    ]));
                }
                if let Some(status) = selected.get_status() {
                    table.push(spacer.clone());
                    for &(heading, value) in &[
                        ("State", &status.state),
                        ("Installation", &status.installdir),
                        ("Size", &convert(status.size)),
                    ] {
                        table.push(Row::new(vec![
                            Cell::from(Span::styled(
                                heading,
                                Style::default().add_modifier(Modifier::BOLD),
                            )),
                            Cell::from(Span::raw(value.clone())),
                        ]));
                    }
                }
                Table::new(table)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(Style::default().fg(Color::White))
                            .title("Detail")
                            .border_type(BorderType::Plain),
                    )
                    .widths(&[Constraint::Percentage(15), Constraint::Percentage(85)])
            }
            None => Table::new(vec![Row::new(vec![Cell::from(Span::raw(
                "No game selected...".to_string(),
            ))])]),
        };
        (list, details)
    }
}
