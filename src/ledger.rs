use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::env;
use std::fs::{self};
use regex::Regex;
use cursive::views::{LinearLayout, Panel, TextView, TextArea, Button, DummyView, ResizedView, ScrollView, Dialog, RadioGroup, SelectView, EditView};
use cursive::traits::*;
use cursive::Cursive;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use cursive::theme::{BaseColor, Color, Style, ColorStyle};
use cursive::utils::markup::StyledString;
use std::path::PathBuf;
use cursive::views::NamedView;

// Add a constant for maximum log lines
const MAX_LOG_LINES: usize = 100;

// Initialize regex pattern for ANSI escape codes
lazy_static! {
    static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]|\x1B\[[0-9;]*m").unwrap();
}

lazy_static! {
    static ref CURRENT_NETWORK: Mutex<String> = Mutex::new("testnet".to_string());
}

// Add this new function to handle ledger connection
fn connect_ledger(s: &mut Cursive) {
    let output = Command::new("solana")
        .arg("address")
        .arg("--keypair")
        .arg("usb://ledger")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                update_logs(s, "✓ Ledger connected successfully!");
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                update_logs(s, &format!("✗ Failed to connect to Ledger: {}", error));
                update_logs(s, &format!("Make sure your Ledger is connected, unlocked, and the Solana app is open."));
            }
        }
        Err(e) => {
            update_logs(s, &format!("✗ Error executing command: {}", e));
        }
    }
}

// Add these functions to update button texts
fn update_x_button_text(s: &mut Cursive, value: &str) {
    let new_label = format!("▼ Select x' ({})", value);
    s.call_on_name("x_button", |view: &mut Button| {
        view.set_label(new_label.clone());
    });
    update_logs(s, &format!("Updated button text to: {}", new_label));
}

fn update_y_button_text(s: &mut Cursive, value: &str) {
    let new_label = format!("▼ Select y' ({})", value);
    s.call_on_name("y_button", |view: &mut Button| {
        view.set_label(new_label.clone());
    });
    update_logs(s, &format!("Updated y button text to: {}", new_label));
}

fn show_wallet_path_select(s: &mut Cursive) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    // Add predefined wallet paths
    select.add_item("Default (x'=1)", "usb://ledger?key=1".to_string());
    select.add_item("m/44'/501'/x' (x'=0)", "usb://ledger?key=0".to_string());
    select.add_item("m/44'/501'/x' (x'=1)", "usb://ledger?key=1".to_string());
    select.add_item("m/44'/501'/x' (x'=2)", "usb://ledger?key=2".to_string());
    select.add_item("m/44'/501'/x' (x'=3)", "usb://ledger?key=3".to_string());
    select.add_item("m/44'/501'/x' (x'=4)", "usb://ledger?key=4".to_string());
    select.add_item("m/44'/501'/x' (x'=5)", "usb://ledger?key=5".to_string());
    select.add_item("m/44'/501'/x' (x'=6)", "usb://ledger?key=6".to_string());
    select.add_item("m/44'/501'/x' (x'=7)", "usb://ledger?key=7".to_string());
    select.add_item("m/44'/501'/x' (x'=8)", "usb://ledger?key=8".to_string());
    select.add_item("m/44'/501'/x' (x'=9)", "usb://ledger?key=9".to_string());

    select.set_on_submit(move |s, path: &String| {
        s.call_on_name("wallet_path_edit", |view: &mut EditView| {
            view.set_content(path);
        });
        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title("Select x' Path")
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

fn show_account_select(s: &mut Cursive) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    let current_value = s.call_on_name("x_button", |button: &mut Button| {
        let label = button.label().to_string();
        if let Some(num_str) = label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<usize>()
            .ok() 
        {
            num_str
        } else {
            1
        }
    }).unwrap_or(1);
    
    update_logs(s, &format!("Current value from button: {}", current_value));
    
    select.add_item("Account 0", "0".to_string());
    select.add_item("Account 1", "1".to_string());
    select.add_item("Account 2", "2".to_string());
    
    select.set_selection(current_value);

    select.set_on_submit(move |s, account: &String| {
        // Update x button and path
        s.call_on_name("wallet_path_text", |view: &mut TextView| {
            let styled_text = StyledString::styled(
                format!("usb://ledger?key={}", account),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            );
            view.set_content(styled_text);
        });
        update_x_button_text(s, account);
        
        // Reset y value to N/A
        update_y_button_text(s, "N/A");
        
        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title("Select Account Index (x')")
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

fn show_address_select(s: &mut Cursive) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    // Get current y' value first
    let current_value = s.call_on_name("y_button", |button: &mut Button| {
        let label = button.label().to_string();
        // Try different ways to get the value
        let value = label.chars()
            .filter(|c| c.is_digit(10) || *c == 'N')
            .collect::<String>();
        
        if value == "N" || value == "NA" {
            0  // Index for "N/A" option
        } else if let Ok(num) = value.parse::<usize>() {
            num + 1  // Add 1 because "N/A" is at index 0
        } else {
            0
        }
    }).unwrap_or(0);
    
    update_logs(s, &format!("Current value from y button: {}", current_value));
    
    // Add items
    select.add_item("N/A (no address index)", "N/A".to_string());
    select.add_item("Address 0", "0".to_string());
    select.add_item("Address 1", "1".to_string());
    select.add_item("Address 2", "2".to_string());
    
    update_logs(s, &format!("Setting y selection to index: {}", current_value));
    select.set_selection(current_value);

    select.set_on_submit(move |s, address: &String| {
        s.call_on_name("wallet_path_text", |view: &mut TextView| {
            let current_path = view.get_content().source().to_string();
            if let Some(x_value) = current_path
                .strip_prefix("usb://ledger?key=")
                .and_then(|s| s.split('/').next()) 
            {
                let new_text = if address == "N/A" {
                    format!("usb://ledger?key={}", x_value)
                } else {
                    format!("usb://ledger?key={}/{}", x_value, address)
                };
                
                let styled_text = StyledString::styled(
                    new_text,
                    ColorStyle::new(
                        Color::Dark(BaseColor::White),
                        Color::Dark(BaseColor::Blue)
                    )
                );
                view.set_content(styled_text);
            }
        });
        update_y_button_text(s, address);
        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title("Select Address Index (y')")
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

// Add these functions for vote key
fn update_vote_x_button_text(s: &mut Cursive, value: &str) {
    let new_label = format!("▼ Select x' ({})", value);
    s.call_on_name("vote_x_button", |view: &mut Button| {
        view.set_label(new_label.clone());
    });
    update_logs(s, &format!("Updated vote button text to: {}", new_label));
}

fn update_vote_y_button_text(s: &mut Cursive, value: &str) {
    let new_label = format!("▼ Select y' ({})", value);
    s.call_on_name("vote_y_button", |view: &mut Button| {
        view.set_label(new_label.clone());
    });
    update_logs(s, &format!("Updated vote y button text to: {}", new_label));
}

fn show_vote_account_select(s: &mut Cursive) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    let current_value = s.call_on_name("vote_x_button", |button: &mut Button| {
        let label = button.label().to_string();
        if let Some(num_str) = label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<usize>()
            .ok() 
        {
            num_str
        } else {
            1
        }
    }).unwrap_or(1);
    
    update_logs(s, &format!("Current value from vote button: {}", current_value));
    
    select.add_item("Account 0", "0".to_string());
    select.add_item("Account 1", "1".to_string());
    select.add_item("Account 2", "2".to_string());
    
    select.set_selection(current_value);

    select.set_on_submit(move |s, account: &String| {
        // Update x button and path
        s.call_on_name("vote_path_text", |view: &mut TextView| {
            let styled_text = StyledString::styled(
                format!("usb://ledger?key={}", account),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            );
            view.set_content(styled_text);
        });
        update_vote_x_button_text(s, account);
        
        // Reset y value to N/A
        update_vote_y_button_text(s, "N/A");
        
        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title("Select Account Index (x')")
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

fn show_vote_address_select(s: &mut Cursive) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    // Get current y' value first
    let current_value = s.call_on_name("vote_y_button", |button: &mut Button| {
        let label = button.label().to_string();
        // Try different ways to get the value
        let value = label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>();
        
        if let Ok(num) = value.parse::<usize>() {
            num + 1  // Add 1 because "N/A" is at index 0
        } else {
            1  // Default to index 1 (Address 0) for vote key
        }
    }).unwrap_or(1);
    
    update_logs(s, &format!("Current value from vote y button: {}", current_value));
    
    // Add items
    select.add_item("N/A (no address index)", "N/A".to_string());
    select.add_item("Address 0", "0".to_string());
    select.add_item("Address 1", "1".to_string());
    select.add_item("Address 2", "2".to_string());
    
    update_logs(s, &format!("Setting vote y selection to index: {}", current_value));
    select.set_selection(current_value);

    select.set_on_submit(move |s, address: &String| {
        s.call_on_name("vote_path_text", |view: &mut TextView| {
            let current_path = view.get_content().source().to_string();
            if let Some(x_value) = current_path
                .strip_prefix("usb://ledger?key=")
                .and_then(|s| s.split('/').next()) 
            {
                let new_text = if address == "N/A" {
                    format!("usb://ledger?key={}", x_value)
                } else {
                    format!("usb://ledger?key={}/{}", x_value, address)
                };
                
                let styled_text = StyledString::styled(
                    new_text,
                    ColorStyle::new(
                        Color::Dark(BaseColor::White),
                        Color::Dark(BaseColor::Blue)
                    )
                );
                view.set_content(styled_text);
            }
        });
        update_vote_y_button_text(s, address);
        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title("Select Address Index (y')")
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

// Create and return the validator view layout
pub fn get_ledger_view() -> LinearLayout {
    let dashboard = Panel::new(LinearLayout::vertical())
        .title("Dashboard")
        .full_width()
        .fixed_height(5)
        .with_name("dashboard");

    // Create config section
    let config = Panel::new(
        LinearLayout::vertical()
            .child(Button::new("Connect Ledger", connect_ledger))
            .child(DummyView.fixed_height(1))
            // VAULT KEY section
            .child(TextView::new("VAULT (ID/WITHDRAW) KEY:"))
            .child(
                LinearLayout::horizontal()
                    .child(
                        Button::new("▼ Select x' (1)", show_account_select)
                            .with_name("x_button")
                            .fixed_width(20)
                    )
                    .child(DummyView.fixed_width(1))
                    .child(
                        Button::new("▼ Select y' (N/A)", show_address_select)
                            .with_name("y_button")
                            .fixed_width(20)
                    )
                    .child(DummyView.fixed_width(1))
                    .child(
                        TextView::new(
                            StyledString::styled(
                                "usb://ledger?key=1",
                                ColorStyle::new(
                                    Color::Dark(BaseColor::White),
                                    Color::Dark(BaseColor::Blue)
                                )
                            )
                        )
                        .with_name("wallet_path_text")
                    )
            )
            .child(DummyView.fixed_height(1))
            // Add VOTE KEY section
            .child(TextView::new("VOTE KEY:"))
            .child(
                LinearLayout::horizontal()
                    .child(
                        Button::new("▼ Select x' (1)", show_vote_account_select)
                            .with_name("vote_x_button")
                            .fixed_width(20)
                    )
                    .child(DummyView.fixed_width(1))
                    .child(
                        Button::new("▼ Select y' (0)", show_vote_address_select)
                            .with_name("vote_y_button")
                            .fixed_width(20)
                    )
                    .child(DummyView.fixed_width(1))
                    .child(
                        TextView::new(
                            StyledString::styled(
                                "usb://ledger?key=1/0",
                                ColorStyle::new(
                                    Color::Dark(BaseColor::White),
                                    Color::Dark(BaseColor::Blue)
                                )
                            )
                        )
                        .with_name("vote_path_text")
                    )
            )
            .child(DummyView.fixed_height(1))
    )
    .title("Config")
    .full_width()
    .min_height(10);

    let logs = Panel::new(
        ScrollView::new(TextView::new(""))
            .scroll_strategy(cursive::view::ScrollStrategy::StickToBottom)
    )
    .title("Logs")
    .with_name("log_view")
    .full_width()
    .min_height(8);

    // Combine sections vertically
    let layout = LinearLayout::vertical()
        .child(dashboard)
        .child(config)
        .child(logs);
    
    layout
}

// Clean ANSI escape sequences from log message
fn clean_log_message(message: &str) -> String {
    ANSI_ESCAPE_RE.replace_all(message, "").to_string()
}

// Update the logs panel with new content
fn update_logs(siv: &mut Cursive, message: &str) {
    // Clean ANSI escape sequences before displaying
    let clean_message = clean_log_message(message);
    
    siv.call_on_name("log_view", |view: &mut Panel<ScrollView<TextView>>| {
        view.get_inner_mut().get_inner_mut().append(&clean_message);
        view.get_inner_mut().get_inner_mut().append("\n");
    });
}

// Add this function to handle wallet path selection changes
fn on_wallet_path_select(s: &mut Cursive, path: &str) {
    s.call_on_name("wallet_path_edit", |view: &mut EditView| {
        view.set_content(path);
    });
}
