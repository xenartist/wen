use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use std::vec::Vec;

struct App {
    logs: Vec<String>,
    scroll: u16,
}

impl App {
    fn new() -> App {
        App { 
            logs: Vec::new(),
            scroll: 0,
        }
    }

    fn add_log(&mut self, message: String) {
        self.logs.push(message);
        self.scroll = self.logs.len() as u16;
    }
}

fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app state
    let mut app = App::new();

    loop {
        // draw UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3), // button area
                    Constraint::Min(0),    // log area
                ].as_ref())
                .split(f.area());

            // create horizontal layout for buttons
            let button_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(15), // first button
                    Constraint::Length(2),  // gap between buttons
                    Constraint::Length(15), // second button
                    Constraint::Min(0),     // remaining space
                ].as_ref())
                .split(chunks[0]);

            // render two buttons
            let button1 = Paragraph::new("[ Button One ]")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Green));
            let button2 = Paragraph::new("[ Button Two ]")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Blue));
            
            f.render_widget(button1, button_chunks[0]);
            f.render_widget(button2, button_chunks[2]);

            // render log area
            let logs = app.logs.join("\n");
            let log_area = Paragraph::new(logs)
                .block(Block::default().title("Event Logs").borders(Borders::ALL))
                .scroll((app.scroll.saturating_sub(chunks[1].height.saturating_sub(2)), 0));
            f.render_widget(log_area, chunks[1]);
        })?;

        // handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Mouse(MouseEvent { kind, column, row, .. }) = event::read()? {
                match kind {
                    MouseEventKind::Down(_) => {
                        // check button 1 click (considering borders)
                        if row == 2 && column >= 1 && column <= 13 {
                            app.add_log(format!("Button One clicked at ({}, {})", column, row));
                        }
                        // check button 2 click (considering borders)
                        else if row == 2 && column >= 17 && column <= 29 {
                            app.add_log(format!("Button Two clicked at ({}, {})", column, row));
                        }
                    }
                    _ => {}
                }
            } else if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // clean up terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
