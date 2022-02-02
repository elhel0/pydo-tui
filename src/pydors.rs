mod pydors;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
    Terminal,
};

enum Event<I> {
    Input(I),
    Tick,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(100);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let menu_titles = vec!["Todo", "Remember", "Add", "Quit"];
    let empty_write = vec![Span::raw("Enter text...")];
    let mut selected = 0;
    let mut position: usize = 0;
    let mut is_writing = false;
    let mut text = vec![];
    let mut pydo: json::JsonValue;

    loop {
        let result = try_get_pydo();
        if !result.1 {
            disable_raw_mode()?;
            terminal.show_cursor()?;
            break;
        }
        pydo = result.0;
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Min(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            let info_menu = if_else(
                is_writing,
                vec![Spans::from(text.to_vec()), Spans::from("".to_string())],
                vec![Spans::from(empty_write.to_vec())],
            )
            .to_vec();

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();
            let select_this = if_else(is_writing, 2, selected);
            let tabs = Tabs::new(menu)
                .select(select_this)
                .block(Block::default().title("Controls").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            let info = Tabs::new(info_menu)
                .block(Block::default().title("Input").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);
            rect.render_widget(info, chunks[1]);
            rect.render_widget(render_pydo(&pydo, selected, position, chunks[2].height), chunks[2]);
        })?;

        if is_writing {
            match rx.recv()? {
                Event::Input(event) => match event.code {
                    KeyCode::Char(c) => match c {
                        _ => {
                            text.push(Span::raw(c.to_string()));
                        }
                    },
                    KeyCode::Backspace => {
                        text.pop();
                    }
                    KeyCode::Esc => {
                        text = vec![];
                        position = 0;
                        is_writing = false;
                    }
                    KeyCode::Enter => {
                        let new_todo = text
                            .iter()
                            .map(|span| {
                                let t: String = span.content.to_string();
                                t
                            })
                            .collect();
                        if selected == 0 {
                            pydors::parse(vec!["pydors".to_string(), "add".to_string(), new_todo]);
                        } else {
                            pydors::parse(vec![
                                "pydors".to_string(),
                                "remember".to_string(),
                                new_todo,
                            ]);
                        }
                        text = vec![];
                        position = 0;
                        is_writing = false;
                    }
                    _ => {}
                },
                Event::Tick => {}
            }
        } else {
            match rx.recv()? {
                Event::Input(event) => match event.code {
                    KeyCode::Char('q') => {
                        disable_raw_mode()?;
                        terminal.show_cursor()?;
                        break;
                    }
                    KeyCode::Char('t') => {
                        selected = 0;
                        position = 0;
                    }
                    KeyCode::Char('r') => {
                        selected = 1;
                        position = 0;
                    }
                    KeyCode::Char('a') => {
                        is_writing = true;
                    }
                    KeyCode::Char('c') => {
                        if selected == 0 {
                            pydors::parse(vec![
                                "pydors".to_string(),
                                "remove".to_string(),
                                "all".to_string(),
                            ]);
                        }
                        position = 0;
                    }
                    KeyCode::Enter => {
                        if selected == 0 && position < pydo["tasks"].len() {
                            pydors::parse(vec![
                                "pydors".to_string(),
                                "complete".to_string(),
                                position.to_string(),
                            ]);
                        }
                    }
                    KeyCode::Backspace => {
                        if selected == 0 && position < pydo["tasks"].len() {
                            pydors::parse(vec![
                                "pydors".to_string(),
                                "remove".to_string(),
                                position.to_string(),
                            ]);
                        } else if selected == 1 && position < pydo["remember-items"].len() {
                            pydors::parse(vec![
                                "pydors".to_string(),
                                "remove-remember".to_string(),
                                position.to_string(),
                            ]);
                        }
                        position = up(position, selected, pydo);
                    }
                    KeyCode::Down => position = down(position, selected, pydo),
                    KeyCode::Up => position = up(position, selected, pydo),
                    KeyCode::Right => {}
                    KeyCode::Left => {}
                    _ => {}
                },
                Event::Tick => {}
            }
        }
    }

    Ok(())
}

fn render_pydo<'a>(pydo: &'a json::JsonValue, selected: usize, position: usize, height: u16) -> Paragraph<'a> {
    let mut todos = vec![];
    let mut i: usize = 0;
    let h: i32 = height.into();
    let p: i32 = position.try_into().unwrap_or(0);
    if p > h - 3 {
        i = (p - (h - 3)).try_into().unwrap_or(0);
    }
    let highlight = Style::default().fg(Color::Yellow);
    loop {
        if selected == 0 && i >= pydo["tasks"].len()
            || selected == 1 && i >= pydo["remember-items"].len()
        {
            break;
        }
        let line: &str;
        let mark: &str;
        line = if_else(
            selected == 0,
            &pydo["tasks"][i]["task"],
            &pydo["remember-items"][i]["item"],
        )
        .as_str()
        .unwrap_or(" ");
        if selected == 0 && pydo["tasks"][i]["completed"] == true {
            mark = " \u{2B24}";
        } else if selected == 0 {
            mark = " \u{25EF}";
        } else {
            mark = "";
        }
        if position == i {
            todos.push(Spans::from(vec![
                Span::styled(mark, highlight),
                Span::styled(" - ", highlight),
                Span::styled(line, highlight),
            ]));
        } else {
            todos.push(Spans::from(vec![
                Span::raw(mark),
                Span::raw(" - "),
                Span::raw(line),
            ]));
        }
        i += 1;
    }
    let grid = Paragraph::new(todos).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .border_type(BorderType::Plain),
    );
    grid
}

fn try_get_pydo() -> (json::JsonValue, bool) {
    let fp = get_path();
    pydors::parse(vec!["".to_string()]);
    if Path::new(&fp).exists() {
        let contents = fs::read_to_string(&fp).unwrap();
        return (json::parse(&contents).unwrap(), true);
    }
    (json::parse("{}").unwrap(), false)
}

fn get_path() -> PathBuf {
    let dir = env::current_dir().unwrap();
    return dir.join("pydo.td");
}

fn up(position: usize, selected: usize, pydo: json::JsonValue) -> usize {
    let as_int: i32 = position.try_into().unwrap_or(0);
    let len: i32 = if_else(
        selected == 0,
        pydo["tasks"].len(),
        pydo["remember-items"].len(),
    )
    .try_into()
    .unwrap_or(0);
    let p = (as_int - 1)
        .try_into()
        .unwrap_or((len - 1).try_into().unwrap_or(0));
    p
}

fn down(position: usize, selected: usize, pydo: json::JsonValue) -> usize {
    let mut p = position + 1;
    if selected == 0 && p == pydo["tasks"].len()
        || selected == 1 && p == pydo["remember-items"].len()
    {
        p = 0;
    }
    p
}

fn if_else<T>(condition: bool, a: T, b: T) -> T {
    if condition {
        return a;
    }
    b
}
