use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::models::TabPage;
use crate::app::App;

pub fn draw_tab_page(f: &mut Frame, area: Rect, tab: &TabPage) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ].as_ref())
        .split(area);

    let config_text = tab.config.join("\n");
    let config_widget = Paragraph::new(config_text)
        .block(Block::default().title("Configuration").borders(Borders::ALL));
    f.render_widget(config_widget, chunks[0]);

    let logs = tab.logs.join("\n");
    let log_widget = Paragraph::new(logs)
        .block(Block::default().title("Logs").borders(Borders::ALL))
        .scroll((tab.scroll.saturating_sub(chunks[1].height.saturating_sub(2)), 0));
    f.render_widget(log_widget, chunks[1]);
}

pub fn draw_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
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
            ListItem::new(item.name.as_str()).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(Block::default().title("Menu").borders(Borders::ALL));
    f.render_widget(menu, chunks[0]);

    // Create right side layout with tabs
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ].as_ref())
        .split(chunks[1]);

    let current_menu = app.current_menu_item();

    // Create horizontal layout for tab buttons
    let tab_button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            current_menu.tabs.iter().map(|_| Constraint::Ratio(1, current_menu.tabs.len() as u32))
                .collect::<Vec<_>>()
        )
        .split(right_chunks[0]);

    // Render tab buttons
    for (i, tab) in current_menu.tabs.iter().enumerate() {
        let button_style = if i == current_menu.active_tab {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let button = Paragraph::new(format!("[ {} ]", tab.name))
            .style(button_style)
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(button, tab_button_chunks[i]);
    }

    // Render active tab content
    if let Some(active_tab) = current_menu.tabs.get(current_menu.active_tab) {
        draw_tab_page(f, right_chunks[1], active_tab);
    }
}