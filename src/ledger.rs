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
}

fn update_y_button_text(s: &mut Cursive, value: &str) {
    let new_label = format!("▼ Select y' ({})", value);
    s.call_on_name("y_button", |view: &mut Button| {
        view.set_label(new_label.clone());
    });
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
            0
        }
    }).unwrap_or(0);
    
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
    
    // Add items
    select.add_item("N/A (no address index)", "N/A".to_string());
    select.add_item("Address 0", "0".to_string());
    select.add_item("Address 1", "1".to_string());
    select.add_item("Address 2", "2".to_string());
    
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
}

fn update_vote_y_button_text(s: &mut Cursive, value: &str) {
    let new_label = format!("▼ Select y' ({})", value);
    s.call_on_name("vote_y_button", |view: &mut Button| {
        view.set_label(new_label.clone());
    });
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
            0
        }
    }).unwrap_or(0);
    
    select.add_item("Account 0", "0".to_string());
    select.add_item("Account 1", "1".to_string());
    select.add_item("Account 2", "2".to_string());
    
    select.set_selection(current_value);

    select.set_on_submit(move |s, account: &String| {
        // Update x button
        update_vote_x_button_text(s, account);
        
        // Reset y to default value (0)
        update_vote_y_button_text(s, "0");

        // Update path text with new x and default y
        s.call_on_name("vote_path_text", |view: &mut TextView| {
            let styled_text = StyledString::styled(
                format!("usb://ledger?key={}/0", account),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            );
            view.set_content(styled_text);
        });

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
    
    // Add items
    select.add_item("N/A (no address index)", "N/A".to_string());
    select.add_item("Address 0", "0".to_string());
    select.add_item("Address 1", "1".to_string());
    select.add_item("Address 2", "2".to_string());
    
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

// Add this function to get public key
fn get_pubkey(path: &str) -> Result<String, String> {
    let output = Command::new("solana")
        .arg("address")
        .arg("-k")
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

// Add this function to handle button click
fn show_pubkey(s: &mut Cursive, path_view_name: &str, pubkey_view_name: &str) {
    if let Some(path) = s.call_on_name(path_view_name, |view: &mut TextView| {
        view.get_content().source().to_string()
    }) {
        match get_pubkey(&path) {
            Ok(pubkey) => {
                s.call_on_name(pubkey_view_name, |view: &mut TextView| {
                    view.set_content(pubkey);
                });
            }
            Err(err) => {
                update_logs(s, &format!("Failed to get public key: {} \nPlease check if the ledger is connected, unlocked and the Solana app is open.", err));
            }
        }
    }
}

// Add helper function to create stake key section
fn create_stake_key_section(
    index: usize,
    default_y: usize,
) -> LinearLayout {
    LinearLayout::vertical()
        .child(TextView::new(format!("STAKE KEY {}:", index)))
        .child(
            LinearLayout::horizontal()
                .child(
                    Button::new("▼ Select x' (0)", move |s| {
                        show_stake_account_select(s, index);
                    })
                    .with_name(format!("stake{}_x_button", index))
                    .fixed_width(20)
                )
                .child(DummyView.fixed_width(1))
                .child(
                    Button::new(format!("▼ Select y' ({})", default_y), move |s| {
                        show_stake_address_select(s, index);
                    })
                    .with_name(format!("stake{}_y_button", index))
                    .fixed_width(20)
                )
                .child(DummyView.fixed_width(1))
                .child(
                    TextView::new(
                        StyledString::styled(
                            format!("usb://ledger?key=0/{}", default_y),
                            ColorStyle::new(
                                Color::Dark(BaseColor::White),
                                Color::Dark(BaseColor::Blue)
                            )
                        )
                    )
                    .with_name(format!("stake{}_path_text", index))
                )
                .child(DummyView.fixed_width(1))
                .child(
                    Button::new("Show Pub Key", move |s| {
                        show_pubkey(s, 
                            &format!("stake{}_path_text", index),
                            &format!("stake{}_pubkey_text", index)
                        );
                    })
                    .fixed_width(15)
                )
                .child(DummyView.fixed_width(1))
                .child(TextView::new("").with_name(format!("stake{}_pubkey_text", index)))
        )
}

// Add functions for stake account selection
fn show_stake_account_select(s: &mut Cursive, stake_index: usize) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    let current_value = s.call_on_name(&format!("stake{}_x_button", stake_index), |button: &mut Button| {
        let label = button.label().to_string();
        if let Some(num_str) = label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<usize>()
            .ok() 
        {
            num_str
        } else {
            0
        }
    }).unwrap_or(0);
    
    select.add_item("Account 0", "0".to_string());
    select.add_item("Account 1", "1".to_string());
    select.add_item("Account 2", "2".to_string());
    
    select.set_selection(current_value);

    let stake_index = stake_index.clone();
    select.set_on_submit(move |s, account: &String| {
        // Update x button
        update_stake_x_button_text(s, stake_index, account);
        
        // Reset y to default value (stake_index)
        let default_y = stake_index.to_string();
        update_stake_y_button_text(s, stake_index, &default_y);

        // Update path text with new x and default y
        s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
            let styled_text = StyledString::styled(
                format!("usb://ledger?key={}/{}", account, default_y),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            );
            view.set_content(styled_text);
        });

        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title(format!("Select Account Index (x') for Stake {}", stake_index))
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

fn show_stake_address_select(s: &mut Cursive, stake_index: usize) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    // Get current y value
    let current_value = s.call_on_name(&format!("stake{}_y_button", stake_index), |button: &mut Button| {
        let label = button.label().to_string();
        if let Some(num_str) = label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<usize>()
            .ok() 
        {
            num_str
        } else {
            stake_index
        }
    }).unwrap_or(stake_index);
    
    select.add_item("Address 0", "0".to_string());
    select.add_item("Address 1", "1".to_string());
    select.add_item("Address 2", "2".to_string());
    select.add_item("Address 3", "3".to_string());
    select.add_item("Address 4", "4".to_string());
    select.add_item("Address 5", "5".to_string());
    
    select.set_selection(current_value);

    let stake_index = stake_index.clone();
    select.set_on_submit(move |s, address: &String| {
        // First get the current path to extract the x value
        let current_x = s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
            let current_path = view.get_content().source().to_string();
            current_path
                .strip_prefix("usb://ledger?key=")
                .and_then(|s| s.split('/').next())
                .unwrap_or("0")
                .to_string()
        }).unwrap_or_else(|| "0".to_string());

        // Update y button
        update_stake_y_button_text(s, stake_index, address);

        // Update path text with current x value and new y value
        s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
            let styled_text = StyledString::styled(
                format!("usb://ledger?key={}/{}", current_x, address),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            );
            view.set_content(styled_text);
        });

        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title(format!("Select Address Index (y') for Stake {}", stake_index))
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

// Helper functions for updating stake buttons and path
fn update_stake_x_button_text(s: &mut Cursive, stake_index: usize, value: &str) {
    let new_label = format!("▼ Select x' ({})", value);
    s.call_on_name(&format!("stake{}_x_button", stake_index), |view: &mut Button| {
        view.set_label(new_label);
    });
}

fn update_stake_y_button_text(s: &mut Cursive, stake_index: usize, value: &str) {
    let new_label = format!("▼ Select y' ({})", value);
    s.call_on_name(&format!("stake{}_y_button", stake_index), |view: &mut Button| {
        view.set_label(new_label);
    });
}

fn update_stake_path(s: &mut Cursive, stake_index: usize) {
    let x_value = s.call_on_name(&format!("stake{}_x_button", stake_index), |button: &mut Button| {
        button.label()
            .strip_prefix("▼ Select x' (")
            .and_then(|s| s.strip_suffix(")"))
            .unwrap_or("0")
            .to_string()
    }).unwrap_or_else(|| "0".to_string());

    let y_value = s.call_on_name(&format!("stake{}_y_button", stake_index), |button: &mut Button| {
        button.label()
            .strip_prefix("▼ Select y' (")
            .and_then(|s| s.strip_suffix(")"))
            .unwrap_or(&stake_index.to_string())
            .to_string()
    }).unwrap_or_else(|| stake_index.to_string());

    s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
        let styled_text = StyledString::styled(
            format!("usb://ledger?key={}/{}", x_value, y_value),
            ColorStyle::new(
                Color::Dark(BaseColor::White),
                Color::Dark(BaseColor::Blue)
            )
        );
        view.set_content(styled_text);
    });
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
            // Add derivation path example
            .child(
                TextView::new(
                    StyledString::styled(
                        "Sample Derivation Path: m/44'/501'/x'/y'",
                        ColorStyle::new(
                            Color::Dark(BaseColor::Yellow),
                            Color::Dark(BaseColor::Black)
                        )
                    )
                )
            )
            .child(DummyView.fixed_height(1))
            // VAULT KEY section
            .child(TextView::new("VAULT (ID/WITHDRAW) KEY:"))
            .child(
                LinearLayout::horizontal()
                    .child(
                        Button::new("▼ Select x' (0)", show_account_select)
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
                                "usb://ledger?key=0",
                                ColorStyle::new(
                                    Color::Dark(BaseColor::White),
                                    Color::Dark(BaseColor::Blue)
                                )
                            )
                        )
                        .with_name("wallet_path_text")
                    )
                    .child(DummyView.fixed_width(1))
                    .child(
                        Button::new("Show Pub Key", move |s| {
                            show_pubkey(s, "wallet_path_text", "wallet_pubkey_text");
                        })
                        .fixed_width(15)
                    )
                    .child(DummyView.fixed_width(1))
                    .child(TextView::new("").with_name("wallet_pubkey_text"))
            )
            .child(DummyView.fixed_height(1))
            // Add VOTE KEY section
            .child(TextView::new("VOTE KEY:"))
            .child(
                LinearLayout::horizontal()
                    .child(
                        Button::new("▼ Select x' (0)", show_vote_account_select)
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
                                "usb://ledger?key=0/0",
                                ColorStyle::new(
                                    Color::Dark(BaseColor::White),
                                    Color::Dark(BaseColor::Blue)
                                )
                            )
                        )
                        .with_name("vote_path_text")
                    )
                    .child(DummyView.fixed_width(1))
                    .child(
                        Button::new("Show Pub Key", move |s| {
                            show_pubkey(s, "vote_path_text", "vote_pubkey_text");
                        })
                        .fixed_width(15)
                    )
                    .child(DummyView.fixed_width(1))
                    .child(TextView::new("").with_name("vote_pubkey_text"))
            )
            .child(DummyView.fixed_height(1))
            // Add STAKE KEYs
            .child(create_stake_key_section(1, 1))
            .child(DummyView.fixed_height(1))
            .child(create_stake_key_section(2, 2))
            .child(DummyView.fixed_height(1))
            .child(create_stake_key_section(3, 3))
            .child(DummyView.fixed_height(1))
            .child(create_stake_key_section(4, 4))
            .child(DummyView.fixed_height(1))
            .child(create_stake_key_section(5, 5))
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
