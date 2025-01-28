use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::env;
use std::fs::{self};
use regex::Regex;
use cursive::views::{LinearLayout, Panel, TextView, TextArea, Button, DummyView, ResizedView, ScrollView, Dialog, RadioGroup};
use cursive::traits::*;
use cursive::Cursive;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use cursive::theme::{BaseColor, Color, Style};
use cursive::utils::markup::StyledString;
use std::path::PathBuf;

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
            }
        }
        Err(e) => {
            update_logs(s, &format!("✗ Error executing command: {}", e));
        }
    }
}

// Create and return the validator view layout
pub fn get_ledger_view() -> LinearLayout {
    let dashboard = Panel::new(LinearLayout::vertical())
        .title("Dashboard")
        .full_width()
        .fixed_height(5)
        .with_name("dashboard");

    // Create config section with Connect button
    let config = Panel::new(
        LinearLayout::vertical()
            .child(Button::new("Connect Ledger", connect_ledger))
            .child(DummyView.fixed_height(1))  // Add some spacing
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
