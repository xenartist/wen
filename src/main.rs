mod app;
mod models;
mod ui;
mod wallets;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui::draw::draw_ui(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Mouse(MouseEvent { kind, column, row, .. }) => {
                    match kind {
                        MouseEventKind::Down(_) => {
                            let total_width = terminal.size()?.width;
                            let menu_width = (total_width * 20) / 100;
                            
                            if column < menu_width {
                                if app.handle_menu_click(row) {
                                    let menu = app.current_menu_item_mut();
                                    if let Some(tab) = menu.tabs.get_mut(menu.active_tab) {
                                        tab.add_log(format!("Selected menu item: {}", menu.name));
                                    }
                                }
                            } else if row <= 2 {
                                let tab_area_width = total_width.saturating_sub(menu_width + 2);
                                let tab_width = tab_area_width / app.current_menu_item().tabs.len() as u16;
                                let relative_column = column.saturating_sub(menu_width + 1);
                                app.handle_tab_click(relative_column, tab_width);
                            }
                        }
                        _ => {}
                    }
                }   
                Event::Key(key) => {   
                    match key.code {
                        KeyCode::Char('q') => break,    
                        KeyCode::Down => {
                            app.selected_menu = (app.selected_menu + 1) % app.menu_items.len();
                        }
                        KeyCode::Up => {
                            app.selected_menu = app.selected_menu.checked_sub(1)
                                .unwrap_or(app.menu_items.len() - 1);
                        }
                        KeyCode::Tab => {
                            let menu = app.current_menu_item_mut();
                            menu.active_tab = (menu.active_tab + 1) % menu.tabs.len();
                        }
                        KeyCode::BackTab => {
                            let menu = app.current_menu_item_mut();
                            menu.active_tab = menu.active_tab.checked_sub(1)
                                .unwrap_or(menu.tabs.len() - 1);
                        }
                        KeyCode::Enter => {
                            let menu = app.current_menu_item_mut();
                            if let Some(tab) = menu.tabs.get_mut(menu.active_tab) {
                                match menu.name.as_str() {
                                    "Solana CLI" => {
                                        match tab.name.as_str() {
                                            "Install CLI" => wallets::solana_cli::handle_install_cli_tab(tab),
                                            _ => tab.add_log(format!("Selected tab: {}", tab.name)),
                                        }
                                    }
                                    _ => tab.add_log(format!("Selected menu item: {}", menu.name)),
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}