use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph, List, ListItem, Tabs},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Span, Line},
    Terminal,
};
use std::io;

// Define tab page structure
struct TabPage {
    name: String,
    config: Vec<String>,  // Configuration data
    logs: Vec<String>,    // Log data for this tab
    scroll: u16,          // Scroll position for logs
}

impl TabPage {
    fn new(name: String) -> Self {
        Self {
            name,
            config: Vec::new(),
            logs: Vec::new(),
            scroll: 0,
        }
    }

    fn add_log(&mut self, message: String) {
        self.logs.push(message);
        self.scroll = self.logs.len() as u16;
    }
}

struct App {
    menu_items: Vec<String>,
    selected_menu: usize,
    tabs: Vec<TabPage>,
    active_tab: usize,
}

impl App {
    fn new() -> Self {
        let menu_items = vec![
            "Menu Item 1".to_string(),
            "Menu Item 2".to_string(),
            "Menu Item 3".to_string(),
        ];
        
        let tabs = vec![
            TabPage::new("Tab 1".to_string()),
            TabPage::new("Tab 2".to_string()),
            TabPage::new("Tab 3".to_string()),
        ];

        Self {
            menu_items,
            selected_menu: 0,
            tabs,
            active_tab: 0,
        }
    }

    fn handle_menu_click(&mut self, row: u16) -> bool {
        // Menu items start from row 1 (considering border)
        if let Some(index) = row.checked_sub(1) {
            if (index as usize) < self.menu_items.len() {
                self.selected_menu = index as usize;
                return true;
            }
        }
        false
    }

    fn handle_tab_click(&mut self, column: u16, total_width: u16) -> bool {
        // Calculate tab width (assuming equal distribution)
        let tab_width = (total_width - 2) / self.tabs.len() as u16;
        if let Some(index) = column.checked_sub(1) {
            let tab_index = index / tab_width;
            if (tab_index as usize) < self.tabs.len() {
                self.active_tab = tab_index as usize;
                return true;
            }
        }
        false
    }
}

// Helper function to draw a tab page
fn draw_tab_page(f: &mut ratatui::Frame, area: Rect, tab: &TabPage) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),  // Config area
            Constraint::Percentage(60),  // Log area
        ].as_ref())
        .split(area);

    // Render config area
    let config_text = tab.config.join("\n");
    let config_widget = Paragraph::new(config_text)
        .block(Block::default().title("Configuration").borders(Borders::ALL));
    f.render_widget(config_widget, chunks[0]);

    // Render log area
    let logs = tab.logs.join("\n");
    let log_widget = Paragraph::new(logs)
        .block(Block::default().title("Logs").borders(Borders::ALL))
        .scroll((tab.scroll.saturating_sub(chunks[1].height.saturating_sub(2)), 0));
    f.render_widget(log_widget, chunks[1]);
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| {
            // Split into left menu and right content area
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),  // Left menu
                    Constraint::Percentage(80),  // Right content
                ].as_ref())
                .split(f.area());

            // Render menu list
            let menu_items: Vec<ListItem> = app.menu_items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let style = if i == app.selected_menu {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(item.as_str()).style(style)
                })
                .collect();

            let menu = List::new(menu_items)
                .block(Block::default().title("Menu").borders(Borders::ALL));
            f.render_widget(menu, chunks[0]);

            // Create right side layout with tabs
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),    // Tab bar
                    Constraint::Min(0),       // Tab content
                ].as_ref())
                .split(chunks[1]);

            // Render tabs
            let tab_titles: Vec<Line> = app.tabs
                .iter()
                .map(|t| Line::from(Span::raw(&t.name)))
                .collect();
            
            let tabs = Tabs::new(tab_titles)
                .block(Block::default().borders(Borders::ALL))
                .select(app.active_tab)
                .style(Style::default())
                .highlight_style(Style::default().fg(Color::Yellow));
            f.render_widget(tabs, right_chunks[0]);

            // Render active tab content
            if let Some(active_tab) = app.tabs.get(app.active_tab) {
                draw_tab_page(f, right_chunks[1], active_tab);
            }
        })?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Mouse(MouseEvent { kind, column, row, .. }) => {
                    match kind {
                        MouseEventKind::Down(_) => {
                            let total_width = terminal.size()?.width;
                            let menu_width = (total_width * 20) / 100;
                            
                            if column < menu_width {
                                // Click in menu area
                                if app.handle_menu_click(row) {
                                    // Add log to current tab when menu item is clicked
                                    if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                                        tab.add_log(format!("Selected menu item: {}", 
                                            app.menu_items[app.selected_menu]));
                                    }
                                }
                            } else {
                                // Click in tab area (only top 3 rows for tab headers)
                                if row <= 2 {
                                    let tab_area_start = menu_width;
                                    let relative_column = column - tab_area_start;
                                    let tab_area_width = total_width - menu_width;
                                    app.handle_tab_click(relative_column, tab_area_width);
                                }
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
                            app.active_tab = (app.active_tab + 1) % app.tabs.len();
                        }
                        KeyCode::BackTab => {
                            app.active_tab = app.active_tab.checked_sub(1)
                                .unwrap_or(app.tabs.len() - 1);
                        }
                        KeyCode::Enter => {
                            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                                tab.add_log(format!("Selected menu item: {}", 
                                    app.menu_items[app.selected_menu]));
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