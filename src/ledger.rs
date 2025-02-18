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
use clipboard::{ClipboardContext, ClipboardProvider};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use crate::encrypt::Encryptor; 

// Add a constant for maximum log lines
const MAX_LOG_LINES: usize = 100;

// Initialize regex pattern for ANSI escape codes
lazy_static! {
    static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]|\x1B\[[0-9;]*m").unwrap();
}

lazy_static! {
    static ref CURRENT_NETWORK: Mutex<String> = Mutex::new("testnet".to_string());
}

// Helper function to update logs when Cursive instance is not available
fn update_logs_static(message: &str) {
    println!("{}", message);  // For now, just print to stdout
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
    
    // Add 10 account options
    for i in 0..10 {
        select.add_item(format!("Account {}", i), i.to_string());
    }
    
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
        
        update_key_tree(s);
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
    
    let current_value = s.call_on_name("y_button", |button: &mut Button| {
        let label = button.label().to_string();
        if let Some(num_str) = label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<usize>()
            .ok() 
        {
            num_str + 1  // Add 1 to account for N/A option
        } else {
            0  // Select N/A
        }
    }).unwrap_or(0);
    
    // Add N/A and 10 account options
    select.add_item("N/A", "N/A".to_string());
    for i in 0..10 {
        select.add_item(format!("Account {}", i), i.to_string());
    }
    
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
    
    // Add 10 account options
    for i in 0..10 {
        select.add_item(format!("Account {}", i), i.to_string());
    }
    
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

        update_key_tree(s);
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
    
    let current_value = s.call_on_name("vote_y_button", |button: &mut Button| {
        let label = button.label().to_string();
        if let Some(num_str) = label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<usize>()
            .ok() 
        {
            num_str + 1  // Add 1 to account for N/A option
        } else {
            0  // Select N/A
        }
    }).unwrap_or(0);
    
    // Add N/A and 10 account options
    select.add_item("N/A", "N/A".to_string());
    for i in 0..10 {
        select.add_item(format!("Account {}", i), i.to_string());
    }
    
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

// Add function to get balance
fn get_balance(key: &str) -> Result<f64, String> {
    let output = Command::new("solana")
        .arg("balance")
        .arg("-k")
        .arg(key)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let balance_str = String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace(" SOL", "");
        
        balance_str.parse::<f64>()
            .map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

// Update show_pubkey function to also show balance
fn show_pubkey(s: &mut Cursive, path_view_name: &str, pubkey_view_name: &str, balance_view_name: &str) {
    if let Some(path) = s.call_on_name(path_view_name, |view: &mut TextView| {
        view.get_content().source().to_string()
    }) {
        update_logs(s, &format!("Getting pubkey for path: {}", path));
        
        match get_pubkey(&path) {
            Ok(pubkey) => {
                update_logs(s, &format!("Got pubkey: {}", pubkey));
                
                // Update pubkey
                s.call_on_name(pubkey_view_name, |view: &mut TextView| {
                    view.set_content(pubkey.clone());
                });
                
                // Get and update balance
                match get_balance(&path) {
                    Ok(balance) => {
                        let balance_text = format!("{:.9} XNT", balance);
                        update_logs(s, &format!("Got balance: {}", balance_text));
                        
                        s.call_on_name(balance_view_name, |view: &mut TextView| {
                            view.set_content(balance_text);
                        });
                    },
                    Err(err) => {
                        update_logs(s, &format!("Failed to get balance: {}", err));
                        s.call_on_name(balance_view_name, |view: &mut TextView| {
                            view.set_content("Error".to_string());
                        });
                    }
                }
            },
            Err(err) => {
                update_logs(s, &format!("Failed to get public key: {}", err));
                update_logs(s, &format!("Make sure your Ledger is connected, unlocked, Solana app is open, and allowed blind signing."));
            }
        }
    }
}

// Add helper function to create stake key section
fn create_stake_key_section(index: usize, default_y: usize) -> LinearLayout {
    LinearLayout::vertical()
        .child(
            LinearLayout::horizontal()
                .child(TextView::new("STAKE KEY:"))
                .child(DummyView.fixed_width(1))
                .child(Button::new("▼ Select Stake Key (1)", show_stake_number_select)
                    .with_name("stake_number_button")
                    .fixed_width(25))
                .child(DummyView.fixed_width(1))
                .child(TextView::new("").with_name(format!("stake{}_pubkey_text", index)))
                .child(DummyView.fixed_width(1))
                .child(TextView::new("").with_name(format!("stake{}_balance", index)).fixed_width(20))
        )
        .child(DummyView.fixed_height(1))
        .child(
            LinearLayout::horizontal()
                .child(Button::new("▼ Select x' (0)", move |s| {
                    show_stake_account_select(s, index);
                })
                .disabled()
                .with_name(&format!("stake{}_x_button", index))
                .fixed_width(20))
                .child(DummyView.fixed_width(1))
                .child(Button::new("▼ Select y' (1)", move |s| {
                    show_stake_address_select(s, index);
                })
                .disabled()
                .with_name(&format!("stake{}_y_button", index))
                .fixed_width(20))
                .child(DummyView.fixed_width(1))
                .child(TextView::new(
                    StyledString::styled(
                        format!("usb://ledger?key=0/{}", default_y),
                        ColorStyle::new(
                            Color::Dark(BaseColor::White),
                            Color::Dark(BaseColor::Blue)
                        )
                    )
                ).with_name(format!("stake{}_path_text", index)))
        )
        .child(DummyView.fixed_height(1))
        .child(
            LinearLayout::horizontal()
                .child(Button::new("Show PubKey & Balance", move |s| {
                    show_pubkey(
                        s,
                        &format!("stake{}_path_text", index),
                        &format!("stake{}_pubkey_text", index),
                        &format!("stake{}_balance", index)
                    );
                }).fixed_width(25))
                .child(DummyView.fixed_width(1))
                .child(Button::new("Copy PubKey", move |s| {
                    if let Some(pubkey) = s.call_on_name(&format!("stake{}_pubkey_text", index), |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }) {
                        if pubkey.is_empty() {
                            update_logs(s, &format!("Please click 'Show PubKey & Balance' button first to get the Stake {} public key", index));
                        } else {
                            if let Ok(mut ctx) = ClipboardContext::new() {
                                if ctx.set_contents(pubkey.clone()).is_ok() {
                                    update_logs(s, &format!("Stake {} PubKey copied to clipboard", index));
                                } else {
                                    update_logs(s, &format!("Failed to copy Stake {} PubKey to clipboard", index));
                                }
                            }
                        }
                    }
                }).fixed_width(15))
                .child(DummyView.fixed_width(1))
                .child(Button::new("Transfer XNT", move |s| {
                    if let Some(pubkey) = s.call_on_name(&format!("stake{}_pubkey_text", index), |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }) {
                        if pubkey.is_empty() {
                            update_logs(s, &format!("Please click 'Show PubKey & Balance' button first to get the Stake {} public key and balance", index));
                        } else {
                            show_transfer_dialog(s, "stake", Some(index));
                        }
                    }
                }).fixed_width(15))
        )
        .child(DummyView.fixed_height(1))
        .child(
            LinearLayout::horizontal() 
                .child(Button::new("Create Stake Account", move |s| {
                    // First check if vault key is available
                    if let Some(vault_pubkey) = s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }) {
                        if vault_pubkey.is_empty() {
                            update_logs(s, "Please click 'Show PubKey & Balance' button first to get the Vault public key");
                            return;
                        }
                    }

                    // Get current stake key number
                    let current_stake_num = s.call_on_name("stake_number_button", |button: &mut Button| {
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

                    // Check if stake pubkey is available
                    if let Some(stake_pubkey) = s.call_on_name(&format!("stake{}_pubkey_text", index), |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }) {
                        if stake_pubkey.is_empty() {
                            update_logs(s, &format!("Please click 'Show PubKey & Balance' button first to get the Stake {} public key", current_stake_num));
                            return;
                        }
                    }

                    // Get stake path and check if account exists
                    if let Some(stake_path) = s.call_on_name(&format!("stake{}_path_text", index), |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }) {
                        let check_output = Command::new("solana")
                            .arg("account")
                            .arg(&stake_path)
                            .output();

                        match check_output {
                            Ok(output) => {
                                if output.status.success() {
                                    update_logs(s, &format!("✗ Account {} already exists. Please use a different stake account.", stake_path));
                                    return;
                                }
                                // Account doesn't exist, we can proceed
                                show_create_stake_account_dialog(s, current_stake_num);
                            }
                            Err(_) => {
                                // Error usually means account doesn't exist, which is what we want
                                show_create_stake_account_dialog(s, current_stake_num);
                            }
                        }
                    }
                }).fixed_width(25))
                .child(DummyView.fixed_width(1))
                .child(Button::new("Check Stake Account", move |s| {
                    // Get current stake key number
                    let current_stake_num = s.call_on_name("stake_number_button", |button: &mut Button| {
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

                    if let Some(pubkey) = s.call_on_name(&format!("stake{}_pubkey_text", index), |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }) {
                        if pubkey.is_empty() {
                            update_logs(s, &format!("Please click 'Show PubKey & Balance' button first to get the Stake {} public key and balance", current_stake_num));
                        } else {
                            // Execute solana stake-account command
                            let output = Command::new("solana")
                                .arg("stake-account")
                                .arg(&pubkey)
                                .output();

                            match output {
                                Ok(output) => {
                                    if output.status.success() {
                                        let result = String::from_utf8_lossy(&output.stdout);
                                        update_logs(s, &format!("Stake {} Account Info:", current_stake_num));
                                        update_logs(s, &result);
                                    } else {
                                        let error = String::from_utf8_lossy(&output.stderr);
                                        update_logs(s, &format!("Failed to get Stake {} account info: {}", current_stake_num, error));
                                    }
                                }
                                Err(e) => {
                                    update_logs(s, &format!("Error executing stake-account command: {}", e));
                                }
                            }
                        }
                    }
                }).fixed_width(25))
                .child(DummyView.fixed_width(1))
                .child(Button::new("Delegate Stake Account", move |s| {
                    show_delegate_dialog(s, index);
                }).fixed_width(25))
        )
        .child(DummyView.fixed_height(1))
        .child(
            LinearLayout::horizontal()
                .child(Button::new("Deactivate Stake Account", move |s| {
                    show_deactivate_dialog(s, index);
                }).fixed_width(28))
        )
}

// Add functions for stake account selection
fn show_stake_account_select(s: &mut Cursive, stake_index: usize) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    let current_value = s.call_on_name(&format!("stake{}_x_button", stake_index), |button: &mut Button| {
        let label = button.label().to_string();
        label.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>()
            .parse::<usize>()
            .ok()
    }).unwrap_or(Some(0)).unwrap_or(0);

    // Add validator options
    for i in 0..=9 {
        select.add_item(format!("Validator {}", i), i.to_string());
    }
    
    select.set_selection(current_value);

    select.set_on_submit(move |s, account: &String| {
        // Update x button
        s.call_on_name(&format!("stake{}_x_button", stake_index), |view: &mut Button| {
            view.set_label(format!("▼ Select x' ({})", account));
        });
        
        // Reset y to default value (1)
        s.call_on_name(&format!("stake{}_y_button", stake_index), |view: &mut Button| {
            view.set_label("▼ Select y' (1)");
        });

        // Get current stake key number
        let stake_num = s.call_on_name("stake_number_button", |button: &mut Button| {
            let label = button.label().to_string();
            label.chars()
                .filter(|c| c.is_digit(10))
                .collect::<String>()
        }).unwrap_or_else(|| "1".to_string());

        // Update path text with new x and default y
        s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
            view.set_content(StyledString::styled(
                format!("usb://ledger?key={}/1", account),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            ));
        });

        // Clear pubkey and balance
        s.call_on_name(&format!("stake{}_pubkey_text", stake_index), |view: &mut TextView| {
            view.set_content("");
        });
        s.call_on_name(&format!("stake{}_balance", stake_index), |view: &mut TextView| {
            view.set_content("");
        });

        update_key_tree(s);
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
    
    // Get current y value from button label
    let current_value = s.call_on_name(&format!("stake{}_y_button", stake_index), |button: &mut Button| {
        let label = button.label().to_string();
        if label.contains("N/A") {
            "N/A".to_string()
        } else {
            label.chars()
                .filter(|c| c.is_digit(10))
                .collect::<String>()
        }
    }).unwrap_or_else(|| "1".to_string());

    // Add address options
    select.add_item("N/A", "N/A".to_string());
    for i in 1..=9 {
        select.add_item(format!("Address {}", i), i.to_string());
    }

    // Set selection based on current value
    if current_value == "N/A" {
        select.set_selection(0);  // N/A is at index 0
    } else if let Ok(num) = current_value.parse::<usize>() {
        select.set_selection(num);  // Add 1 because N/A is the first item
    }

    select.set_on_submit(move |s, address: &String| {
        // Update y button
        s.call_on_name(&format!("stake{}_y_button", stake_index), |view: &mut Button| {
            view.set_label(format!("▼ Select y' ({})", address));
        });

        // Get current x value
        let x_value = s.call_on_name(&format!("stake{}_x_button", stake_index), |button: &mut Button| {
            let label = button.label().to_string();
            label.chars()
                .filter(|c| c.is_digit(10))
                .collect::<String>()
        }).unwrap_or_else(|| "0".to_string());

        // Update path text based on whether address is N/A
        s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
            let path = if address == "N/A" {
                format!("usb://ledger?key={}", x_value)
            } else {
                format!("usb://ledger?key={}/{}", x_value, address)
            };
            
            view.set_content(StyledString::styled(
                path,
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            ));
        });

        // Clear pubkey and balance
        s.call_on_name(&format!("stake{}_pubkey_text", stake_index), |view: &mut TextView| {
            view.set_content("");
        });
        s.call_on_name(&format!("stake{}_balance", stake_index), |view: &mut TextView| {
            view.set_content("");
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
        .fixed_height(2)
        .with_name("dashboard");

    let config = Panel::new(
        ScrollView::new(
            LinearLayout::vertical()
                // Simplified validator selector
                .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("VALIDATOR ID: "))
                            .child(Button::new("▼ Select Validator (0)", show_validator_select)
                                .with_name("validator_button")
                                .fixed_width(25))
                )
                // Add IDENTITY KEY section
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(
                            LinearLayout::horizontal()
                                .child(TextView::new("IDENTITY KEY:"))
                                .child(DummyView.fixed_width(1))
                                .child(TextView::new("").with_name("identity_pubkey_text"))
                        )
                        .child(DummyView.fixed_height(1))
                        .child(
                            LinearLayout::horizontal()
                                .child(Button::new("Show Pubkey & Balance", |s| {
                                    show_identity_info(s);
                                }).fixed_width(25))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("Copy Pubkey", |s| {
                                    copy_identity_pubkey(s);
                                }).fixed_width(15))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("Transfer XNT", |s| {
                                    show_identity_transfer_dialog(s);
                                }).fixed_width(15))
                        )
                        .child(DummyView.fixed_height(1))
                        .child(Button::new("Create Identity Account", |s| {
                            show_create_identity_dialog(s);
                        }).fixed_width(28))
                ))
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
                // VAULT KEY section with panel
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(
                            LinearLayout::horizontal()
                                .child(TextView::new("VAULT (ID/WITHDRAW) KEY:"))
                                .child(DummyView.fixed_width(1))
                                .child(TextView::new("").with_name("wallet_pubkey_text"))
                                .child(DummyView.fixed_width(1))
                                .child(TextView::new("").with_name("vault_balance").fixed_width(20))                          
                        )
                        .child(DummyView.fixed_height(1))
                        .child(
                            LinearLayout::horizontal()
                                .child(Button::new("▼ Select x' (0)", show_account_select)
                                    .disabled()
                                    .with_name("x_button")
                                    .fixed_width(20))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("▼ Select y' (N/A)", show_address_select)
                                    .disabled()
                                    .with_name("y_button")
                                    .fixed_width(20))
                                .child(DummyView.fixed_width(1))
                                .child(TextView::new(
                                    StyledString::styled(
                                        "usb://ledger?key=0",
                                        ColorStyle::new(
                                            Color::Dark(BaseColor::White),
                                            Color::Dark(BaseColor::Blue)
                                        )
                                    )
                                ).with_name("wallet_path_text"))
                        )
                        .child(DummyView.fixed_height(1))
                        .child(
                            LinearLayout::horizontal()
                                .child(Button::new("Show PubKey & Balance", move |s| {
                                    show_pubkey(s, "wallet_path_text", "wallet_pubkey_text", "vault_balance");
                                }).fixed_width(25))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("Copy PubKey", |s| {
                                    if let Some(pubkey) = s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
                                        view.get_content().source().to_string()
                                    }) {
                                        if pubkey.is_empty() {
                                            update_logs(s, "Please click 'Show PubKey & Balance' button first to get the public key");
                                        } else {
                                            if let Ok(mut ctx) = ClipboardContext::new() {
                                                if ctx.set_contents(pubkey.clone()).is_ok() {

                                                    update_logs(s, "Vault PubKey copied to clipboard");
                                                } else {
                                                    update_logs(s, "Failed to copy Vault PubKey to clipboard");
                                                }
                                            }
                                        }
                                    }
                                }).fixed_width(15))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("Transfer XNT", |s| {
                                    if let Some(pubkey) = s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
                                        view.get_content().source().to_string()
                                    }) {
                                        if pubkey.is_empty() {
                                            update_logs(s, "Please click 'Show PubKey & Balance' button first to get the public key and balance");
                                        } else {
                                            show_transfer_dialog(s, "vault", None);
                                        }
                                    }
                                }).fixed_width(15))
                        )
                ))
                // VOTE KEY section
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(
                            LinearLayout::horizontal()
                                .child(TextView::new("VOTE KEY:"))
                                .child(DummyView.fixed_width(1))
                                .child(TextView::new("").with_name("vote_pubkey_text"))
                                .child(DummyView.fixed_width(1))
                                .child(TextView::new("").with_name("vote_balance").fixed_width(20))
                        )
                        .child(DummyView.fixed_height(1))
                        .child(
                            LinearLayout::horizontal()
                                .child(Button::new("▼ Select x' (0)", show_vote_account_select)
                                    .disabled()
                                    .with_name("vote_x_button")
                                    .fixed_width(20))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("▼ Select y' (0)", show_vote_address_select)
                                    .disabled()
                                    .with_name("vote_y_button")
                                    .fixed_width(20))
                                .child(DummyView.fixed_width(1))
                                .child(TextView::new(
                                    StyledString::styled(
                                        "usb://ledger?key=0/0",
                                        ColorStyle::new(
                                            Color::Dark(BaseColor::White),
                                            Color::Dark(BaseColor::Blue)
                                        )
                                    )
                                ).with_name("vote_path_text"))
                        )
                        .child(DummyView.fixed_height(1))
                        .child(
                            LinearLayout::horizontal()
                                .child(Button::new("Show PubKey & Balance", move |s| {
                                    show_pubkey(s, "vote_path_text", "vote_pubkey_text", "vote_balance");
                                }).fixed_width(25))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("Copy PubKey", |s| {
                                    if let Some(pubkey) = s.call_on_name("vote_pubkey_text", |view: &mut TextView| {
                                        view.get_content().source().to_string()
                                    }) {
                                        if pubkey.is_empty() {
                                            update_logs(s, "Please click 'Show PubKey & Balance' button first to get the public key");
                                        } else {
                                            if let Ok(mut ctx) = ClipboardContext::new() {
                                                if ctx.set_contents(pubkey.clone()).is_ok() {

                                                    update_logs(s, "Vote PubKey copied to clipboard");
                                                } else {
                                                    update_logs(s, "Failed to copy Vote PubKey to clipboard");
                                                }
                                            }
                                        }
                                    }
                                }).fixed_width(15))
                                .child(DummyView.fixed_width(1))
                                .child(Button::new("Transfer XNT", |s| {
                                    if let Some(pubkey) = s.call_on_name("vote_pubkey_text", |view: &mut TextView| {
                                        view.get_content().source().to_string()
                                    }) {
                                        if pubkey.is_empty() {
                                            update_logs(s, "Please click 'Show PubKey & Balance' button first to get the public key and balance");
                                        } else {
                                            show_transfer_dialog(s, "vote", None);
                                        }
                                    }
                                }).fixed_width(15))
                        )
                ))
                // STAKE KEYs
                .child(Panel::new(
                    LinearLayout::vertical()
                        .child(create_stake_key_section(1, 1))
                ))
        )
    )
    .title("Configuration")
    .full_width()
    .max_height(40);  // This will now be the height of the scrollable area

    // Create the new key tree panel with default tree structure
    let key_tree = Panel::new(
        LinearLayout::vertical()
            .child(TextView::new("Key Derivation Path Tree:").style(ColorStyle::title_secondary()))
            .child(DummyView.fixed_height(1))
            // Add example section in yellow color
            .child(TextView::new("x' (XXXX)").style(ColorStyle::new(
                Color::Dark(BaseColor::Yellow),
                Color::Dark(BaseColor::Black)
            )))
            .child(TextView::new("└── y' (YYYY)").style(ColorStyle::new(
                Color::Dark(BaseColor::Yellow),
                Color::Dark(BaseColor::Black)
            )))
            .child(DummyView.fixed_height(1))
            // Add default tree structure
            .child(TextView::new("\
0 (VAULT)
└── 0 (VOTE)
└── 1 (STAKE 1)
└── 2 (STAKE 2)
└── 3 (STAKE 3)
└── 4 (STAKE 4)
└── 5 (STAKE 5)
└── 6 (STAKE 6)
└── 7 (STAKE 7)
└── 8 (STAKE 8)
└── 9 (STAKE 9)").with_name("key_tree_view"))
    )
    .title("Key Tree")
    .fixed_width(30);

    let logs = Panel::new(
        ScrollView::new(TextView::new(""))
            .scroll_strategy(cursive::view::ScrollStrategy::StickToBottom)
            .with_name("logs")
    )
    .title("Logs")
    .full_width()
    .fixed_height(10);

    // Arrange panels in the main layout
    LinearLayout::vertical()
        .child(dashboard)
        .child(
            LinearLayout::horizontal()  // New horizontal layout to hold config and key tree
                .child(ResizedView::with_full_screen(config))
                .child(key_tree)
        )
        .child(logs)
}

// Clean ANSI escape sequences from log message
fn clean_log_message(message: &str) -> String {
    ANSI_ESCAPE_RE.replace_all(message, "").to_string()
}

// Update the logs panel with new content
fn update_logs(s: &mut Cursive, message: &str) {
    // Clean ANSI escape sequences before displaying
    let clean_message = clean_log_message(message);
    
    s.call_on_name("logs", |view: &mut ScrollView<TextView>| {
        view.get_inner_mut().append(&clean_message);
        view.get_inner_mut().append("\n");
    });
}

// Add this function to handle wallet path selection changes
fn on_wallet_path_select(s: &mut Cursive, path: &str) {
    s.call_on_name("wallet_path_edit", |view: &mut EditView| {
        view.set_content(path);
    });
}

// Add function to show validator select dialog
fn show_validator_select(s: &mut Cursive) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    let current_value = s.call_on_name("validator_button", |button: &mut Button| {
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
    
    // Add validator options (0-9)
    for i in 0..10 {
        select.add_item(format!("Validator {}", i), i.to_string());
    }
    
    select.set_selection(current_value);

    select.set_on_submit(move |s, validator: &String| {
        // Update validator button text
        s.call_on_name("validator_button", |view: &mut Button| {
            view.set_label(format!("▼ Validator ({})", validator));
        });
        
        // Clear all balances and pubkeys
        // Clear vault key info
        s.call_on_name("vault_balance", |view: &mut TextView| {
            view.set_content("");
        });
        s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
            view.set_content("");
        });
        
        // Clear vote key info
        s.call_on_name("vote_balance", |view: &mut TextView| {
            view.set_content("");
        });
        s.call_on_name("vote_pubkey_text", |view: &mut TextView| {
            view.set_content("");
        });
        
        // Clear stake keys info
        for i in 1..=5 {
            s.call_on_name(&format!("stake{}_balance", i), |view: &mut TextView| {
                view.set_content("");
            });
            s.call_on_name(&format!("stake{}_pubkey_text", i), |view: &mut TextView| {
                view.set_content("");
            });
        }

        // Reset validator name to empty and ensure it's disabled
        s.call_on_name("validator_name", |view: &mut EditView| {
            view.set_content("");
            view.disable();  // Disable editing
        });
        
        // Reset edit button to "Edit"
        s.call_on_name("name_edit_button", |button: &mut Button| {
            button.set_label("Edit");
        });
        
        // Update all x buttons with new default value
        s.call_on_name("x_button", |view: &mut Button| {
            view.set_label(format!("▼ Select x' ({})", validator));
        });
        s.call_on_name("vote_x_button", |view: &mut Button| {
            view.set_label(format!("▼ Select x' ({})", validator));
        });
        
        // Reset y buttons to default values
        s.call_on_name("y_button", |view: &mut Button| {
            view.set_label("▼ Select y' (N/A)");
        });
        s.call_on_name("vote_y_button", |view: &mut Button| {
            view.set_label("▼ Select y' (0)");
        });
        
        // Update stake buttons
        for i in 1..=5 {
            // Update x button
            s.call_on_name(&format!("stake{}_x_button", i), |view: &mut Button| {
                view.set_label(format!("▼ Select x' ({})", validator));
            });
            // Reset y button to default (index number)
            s.call_on_name(&format!("stake{}_y_button", i), |view: &mut Button| {
                view.set_label(format!("▼ Select y' ({})", i));
            });
        }
        
        // Update all paths with new x value and default y values
        s.call_on_name("wallet_path_text", |view: &mut TextView| {
            view.set_content(StyledString::styled(
                format!("usb://ledger?key={}", validator),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            ));
        });
        s.call_on_name("vote_path_text", |view: &mut TextView| {
            view.set_content(StyledString::styled(
                format!("usb://ledger?key={}/0", validator),
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            ));
        });
        for i in 1..=5 {
            s.call_on_name(&format!("stake{}_path_text", i), |view: &mut TextView| {
                view.set_content(StyledString::styled(
                    format!("usb://ledger?key={}/{}", validator, i),
                    ColorStyle::new(
                        Color::Dark(BaseColor::White),
                        Color::Dark(BaseColor::Blue)
                    )
                ));
            });
        }

        // Reset stake key button to default (1)
        s.call_on_name("stake_number_button", |view: &mut Button| {
            view.set_label("▼ Select Stake Key (1)");
        });
        
        // Update stake x button with new validator value
        s.call_on_name("stake1_x_button", |view: &mut Button| {
            view.set_label(format!("▼ Select x' ({})", validator));
        });

        // Reset stake y button to default (1)
        s.call_on_name("stake1_y_button", |view: &mut Button| {
            view.set_label("▼ Select y' (1)");
        });
        
        // Update stake path with new validator value
        s.call_on_name("stake1_path_text", |view: &mut TextView| {
            view.set_content(StyledString::styled(
                format!("usb://ledger?key={}/1", validator), 
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            ));
        });

        // Clear stake pubkey and balance
        s.call_on_name("stake1_pubkey_text", |view: &mut TextView| {
            view.set_content("");
        });
        s.call_on_name("stake1_balance", |view: &mut TextView| {
            view.set_content("");
        });

        // Log the change
        update_logs(s, &format!("Switched to Validator {}", validator));

        update_key_tree(s);
        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title("Select Validator")
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

// Add function to toggle between edit and save modes
fn toggle_name_edit(s: &mut Cursive) {
    let is_edit_mode = s.call_on_name("name_edit_button", |button: &mut Button| {
        button.label() == "Edit"
    }).unwrap_or(false);

    if is_edit_mode {
        // Switch to Save mode
        s.call_on_name("name_edit_button", |button: &mut Button| {
            button.set_label("Save");
        });
        s.call_on_name("validator_name", |view: &mut EditView| {
            view.enable();  // Enable editing
            view.take_focus(cursive::direction::Direction::none()).unwrap();
        });
    } else {
        // Switch to Edit mode
        s.call_on_name("name_edit_button", |button: &mut Button| {
            button.set_label("Edit");
        });
        s.call_on_name("validator_name", |view: &mut EditView| {
            view.disable();  // Disable editing
        });
    }
}

// Add transfer dialog function
fn show_transfer_dialog(s: &mut Cursive, source_type: &str, source_index: Option<usize>) {
    let source_type = source_type.to_string();  // Convert to owned String at the start
    let title = match source_type.as_str() {
        "vault" => "Vault Key".to_string(),
        "vote" => "Vote Key".to_string(),
        "stake" => format!("Stake Key {}", source_index.unwrap_or(0)),
        _ => "Unknown Key".to_string()
    };

    // Get source address and balance
    let (source_address, source_balance) = match source_type.as_str() {
        "vault" => (
            s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
                view.get_content().source().to_string()
            }).unwrap_or_default(),
            s.call_on_name("vault_balance", |view: &mut TextView| {
                view.get_content().source().to_string()
            }).unwrap_or_default()
        ),
        "vote" => (
            s.call_on_name("vote_pubkey_text", |view: &mut TextView| {
                view.get_content().source().to_string()
            }).unwrap_or_default(),
            s.call_on_name("vote_balance", |view: &mut TextView| {
                view.get_content().source().to_string()
            }).unwrap_or_default()
        ),
        "stake" => (
            s.call_on_name(&format!("stake{}_pubkey_text", source_index.unwrap_or(0)), |view: &mut TextView| {
                view.get_content().source().to_string()
            }).unwrap_or_default(),
            s.call_on_name(&format!("stake{}_balance", source_index.unwrap_or(0)), |view: &mut TextView| {
                view.get_content().source().to_string()
            }).unwrap_or_default()
        ),
        _ => (String::new(), String::new())
    };

    let dialog = Dialog::new()
        .title(format!("Transfer XNT from {}", title))
        .content(
            LinearLayout::vertical()
                .child(TextView::new("From Address:"))
                .child(TextView::new(source_address.clone()))
                .child(DummyView.fixed_height(1))
                .child(TextView::new(format!("Available Balance: {}", source_balance)))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Recipient Address:"))
                .child(EditView::new()
                    .with_name("recipient_address")
                    .fixed_width(64))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Amount (XNT):"))
                .child(EditView::new()
                    .with_name("transfer_amount")
                    .fixed_width(20))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Transfer", {
            let source_type = source_type.clone();  // Clone before moving into closure
            move |s| {
                let recipient = s.call_on_name("recipient_address", |view: &mut EditView| {
                    view.get_content().to_string()
                }).unwrap_or_default();
                
                let amount = s.call_on_name("transfer_amount", |view: &mut EditView| {
                    view.get_content().to_string()
                }).unwrap_or_default();

                // Validate inputs
                if recipient.is_empty() || amount.is_empty() {
                    show_error_dialog(s, "Please fill in all fields");
                    return;
                }

                // Validate amount format
                match amount.parse::<f64>() {
                    Ok(amount) if amount <= 0.0 => {
                        show_error_dialog(s, "Amount must be greater than 0");
                        return;
                    }
                    Err(_) => {
                        show_error_dialog(s, "Invalid amount format");
                        return;
                    }
                    _ => {}
                }

                // Get source path based on key type
                let path = match source_type.as_str() {
                    "vault" => s.call_on_name("wallet_path_text", |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }),
                    "vote" => s.call_on_name("vote_path_text", |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }),
                    "stake" => s.call_on_name(&format!("stake{}_path_text", source_index.unwrap_or(0)), |view: &mut TextView| {
                        view.get_content().source().to_string()
                    }),
                    _ => None
                }.unwrap_or_default();

                // Show confirmation dialog
                show_transfer_confirmation(s, &path, &recipient, &amount, source_type.clone(), source_index);
            }
        });

    s.add_layer(dialog);
}

// Add confirmation dialog
fn show_transfer_confirmation(s: &mut Cursive, from_path: &str, to_address: &str, amount: &str, source_type: String, source_index: Option<usize>) {
    let source_type = source_type.clone();  // Clone before moving into closure
    let from_path = from_path.to_string();
    let to_address = to_address.to_string();
    let amount = amount.to_string();

    // Get source pubkey
    let from_pubkey = match source_type.as_str() {
        "vault" => s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
            view.get_content().source().to_string()
        }),
        "vote" => s.call_on_name("vote_pubkey_text", |view: &mut TextView| {
            view.get_content().source().to_string()
        }),
        "stake" => s.call_on_name(&format!("stake{}_pubkey_text", source_index.unwrap_or(0)), |view: &mut TextView| {
            view.get_content().source().to_string()
        }),
        _ => None
    }.unwrap_or_default();

    let dialog = Dialog::new()
        .title("Confirm Transfer")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Please confirm the transfer details:"))
                .child(DummyView.fixed_height(1))
                .child(TextView::new(format!("From Path: {}", from_path)))
                .child(TextView::new(format!("From PubKey: {}", from_pubkey)))
                .child(TextView::new(format!("To: {}", to_address)))
                .child(TextView::new(format!("Amount: {} XNT", amount)))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Confirm", move |s| {
            s.pop_layer();  // Close confirmation dialog
            s.pop_layer();  // Close transfer dialog
            execute_transfer(s, &from_path, &to_address, &amount, &source_type, source_index);
        });

    s.add_layer(dialog);
}

// Add error dialog
fn show_error_dialog(s: &mut Cursive, message: &str) {
    s.add_layer(
        Dialog::new()
            .title("Error")
            .content(TextView::new(message))
            .button("OK", |s| { s.pop_layer(); })
    );
}

// Execute transfer
fn execute_transfer(s: &mut Cursive, from_path: &str, to_address: &str, amount: &str, source_type: &str, source_index: Option<usize>) {
    update_logs(s, &format!("Executing transfer of {} XNT from {} to {}", amount, from_path, to_address));

    // Execute solana transfer command
    let output = Command::new("solana")
        .arg("transfer")
        .arg("--keypair")
        .arg(from_path)
        .arg(to_address)
        .arg(amount)
        .arg("--allow-unfunded-recipient")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                update_logs(s, "✓ Transfer completed successfully!");
                
                // Refresh balance after transfer
                match source_type {
                    "vault" => show_pubkey(s, "wallet_path_text", "wallet_pubkey_text", "vault_balance"),
                    "vote" => show_pubkey(s, "vote_path_text", "vote_pubkey_text", "vote_balance"),
                    "stake" => {
                        if let Some(index) = source_index {
                            show_pubkey(
                                s,
                                &format!("stake{}_path_text", index),
                                &format!("stake{}_pubkey_text", index),
                                &format!("stake{}_balance", index)
                            );
                        }
                    },
                    _ => {}
                }
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                update_logs(s, &format!("✗ Transfer failed: {}", error));
            }
        }
        Err(e) => {
            update_logs(s, &format!("✗ Error executing transfer command: {}", e));
        }
    }
}

// Add new function to show create stake account dialog
fn show_create_stake_account_dialog(s: &mut Cursive, stake_index: usize) {
    // Get vault key info
    let vault_path = s.call_on_name("wallet_path_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();
    
    let vault_pubkey = s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    let vault_balance = s.call_on_name("vault_balance", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    // Get stake key info using fixed index 1 (since we only have one stake key section)
    let stake_path = s.call_on_name("stake1_path_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    let stake_pubkey = s.call_on_name("stake1_pubkey_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    let dialog = Dialog::new()
        .title(format!("Create Stake Account {}", stake_index))
        .content(
            LinearLayout::vertical()
                .child(TextView::new("From VAULT (ID/WITHDRAW) KEY:"))
                .child(TextView::new(format!("Path: {}", vault_path)))
                .child(TextView::new(format!("PubKey: {}", vault_pubkey)))
                .child(TextView::new(format!("Available Balance: {}", vault_balance)))
                .child(DummyView.fixed_height(1))
                .child(TextView::new(format!("To Stake Account {}:", stake_index)))
                .child(TextView::new(format!("Path: {}", stake_path)))
                .child(TextView::new(format!("PubKey: {}", stake_pubkey)))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Stake Amount (XNT):"))
                .child(EditView::new()
                    .content("1")
                    .with_name("stake_amount")
                    .fixed_width(20))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Create", move |s| {
            let amount = s.call_on_name("stake_amount", |view: &mut EditView| {
                view.get_content().to_string()
            }).unwrap_or_default();

            show_create_stake_confirmation(s, &vault_path, &stake_path, &amount, stake_index);
        });

    s.add_layer(dialog);
}

// Add confirmation dialog
fn show_create_stake_confirmation(s: &mut Cursive, vault_path: &str, stake_path: &str, amount: &str, stake_index: usize) {
    let dialog = Dialog::new()
        .title("Confirm Create Stake Account")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Please confirm the stake account creation:"))
                .child(DummyView.fixed_height(1))
                .child(TextView::new(format!("From: {}", vault_path)))
                .child(TextView::new(format!("To: {}", stake_path)))
                .child(TextView::new(format!("Amount: {} XNT", amount)))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Confirm", {
            let vault_path = vault_path.to_string();
            let stake_path = stake_path.to_string();
            let amount = amount.to_string();
            move |s| {
                s.pop_layer();  // Close confirmation dialog
                s.pop_layer();  // Close create dialog
                execute_create_stake_account(s, &vault_path, &stake_path, &amount, stake_index);
            }
        });

    s.add_layer(dialog);
}

// Add execute function
fn execute_create_stake_account(s: &mut Cursive, vault_path: &str, stake_path: &str, amount: &str, stake_index: usize) {
    update_logs(s, &format!("Creating stake account {} with {} XNT...", stake_index, amount));

    let output = Command::new("solana")
        .arg("create-stake-account")
        .arg(stake_path)
        .arg(amount)
        .arg("--keypair")
        .arg(vault_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                update_logs(s, "✓ Stake account created successfully!");
                
                // Refresh balances
                show_pubkey(s, "wallet_path_text", "wallet_pubkey_text", "vault_balance");
                show_pubkey(
                    s,
                    &format!("stake{}_path_text", stake_index),
                    &format!("stake{}_pubkey_text", stake_index),
                    &format!("stake{}_balance", stake_index)
                );
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                update_logs(s, &format!("✗ Failed to create stake account: {}", error));
            }
        }
        Err(e) => {
            update_logs(s, &format!("✗ Error executing create-stake-account command: {}", e));
        }
    }
}

fn show_stake_number_select(s: &mut Cursive) {
    let mut select = SelectView::new()
        .h_align(cursive::align::HAlign::Left)
        .autojump();
    
    let current_value = s.call_on_name("stake_number_button", |button: &mut Button| {
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
    
    // Add stake number options (1-9)
    for i in 1..=9 {
        select.add_item(format!("Stake Key {}", i), i.to_string());
    }
    
    select.set_selection(current_value - 1);

    select.set_on_submit(move |s, stake_num: &String| {
        // Get x value first
        let x_button_label = s.call_on_name("stake1_x_button", |button: &mut Button| {
            button.label().to_string()
        }).unwrap_or_else(|| "▼ Select x' (0)".to_string());

        // Extract x value using a more robust approach
        let x_value = x_button_label
            .chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>();

        let x_value = if x_value.is_empty() { "0".to_string() } else { x_value };

        // Update stake number button text
        s.call_on_name("stake_number_button", |view: &mut Button| {
            view.set_label(format!("▼ Select Stake Key ({})", stake_num));
        });
        
        // Update y button with the stake number
        s.call_on_name("stake1_y_button", |view: &mut Button| {
            view.set_label(format!("▼ Select y' ({})", stake_num));
        });
        
        // Update and log path text
        let path = format!("usb://ledger?key={}/{}", x_value, stake_num);
        update_logs(s, &format!("Setting path to: {}", path));
        
        s.call_on_name("stake1_path_text", |view: &mut TextView| {
            view.set_content(StyledString::styled(
                path,
                ColorStyle::new(
                    Color::Dark(BaseColor::White),
                    Color::Dark(BaseColor::Blue)
                )
            ));
        });

        // Clear pubkey and balance
        s.call_on_name("stake1_pubkey_text", |view: &mut TextView| {
            view.set_content("");
        });
        s.call_on_name("stake1_balance", |view: &mut TextView| {
            view.set_content("");
        });
        
        s.pop_layer();
    });

    s.add_layer(
        Dialog::around(select)
            .title("Select Stake Key Number")
            .button("Cancel", |s| { s.pop_layer(); })
    );
}

// Function to update the key tree view
fn update_key_tree(s: &mut Cursive) {
    let mut tree = String::new();

    // Get Vault Key x value from path
    let vault_x = s.call_on_name("wallet_path_text", |view: &mut TextView| {
        let path = view.get_content().source().to_string();
        if path.is_empty() {
            "0".to_string()
        } else {
            path.split("key=").nth(1)
                .unwrap_or("0")
                .to_string()
        }
    }).unwrap_or_else(|| "0".to_string());

    // Get Vote Key y value from path
    let vote_y = s.call_on_name("vote_path_text", |view: &mut TextView| {
        let path = view.get_content().source().to_string();
        if path.is_empty() {
            "0".to_string()
        } else {
            path.split('/')
                .last()
                .unwrap_or("0")
                .to_string()
        }
    }).unwrap_or_else(|| "0".to_string());

    // Build tree structure
    tree.push_str(&format!("{} (VAULT)\n", vault_x));
    tree.push_str(&format!("└── {} (VOTE)\n", vote_y));

    // Add all Stake Keys with y values
    for i in 1..=9 {
        let stake_y = s.call_on_name(&format!("stake{}_path_text", i), |view: &mut TextView| {
            let path = view.get_content().source().to_string();
            if path.is_empty() {
                i.to_string()  // Default y value matches the stake key number
            } else {
                path.split('/')
                    .last()
                    .unwrap_or(&i.to_string())
                    .to_string()
            }
        }).unwrap_or_else(|| i.to_string());  // Default to stake key number

        tree.push_str(&format!("└── {} (STAKE {})\n", stake_y, i));
    }

    // Update the tree view
    s.call_on_name("key_tree_view", |view: &mut TextView| {
        view.set_content(tree);
    });
}

fn show_delegate_dialog(s: &mut Cursive, stake_index: usize) {
    // Get stake key path
    let stake_path = s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    // Get vote key path
    let vote_path = s.call_on_name("vote_path_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    // Get vault key path
    let vault_path = s.call_on_name("wallet_path_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    // Get stake pubkey
    let stake_pubkey = s.call_on_name(&format!("stake{}_pubkey_text", stake_index), |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    // Get vote pubkey
    let vote_pubkey = s.call_on_name("vote_pubkey_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    // Get vault pubkey
    let vault_pubkey = s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    let dialog = Dialog::new()
        .title("Delegate Stake Account")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Delegate From:"))
                .child(TextView::new(format!("Stake Key: {}", stake_pubkey)))
                .child(TextView::new(format!("Path: {}", stake_path)))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Delegate To:"))
                .child(TextView::new(format!("Vote Key: {}", vote_pubkey)))
                .child(TextView::new(format!("Path: {}", vote_path)))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Stake/Withdraw Auth:"))
                .child(TextView::new(format!("Vault Key: {}", vault_pubkey)))
                .child(TextView::new(format!("Path: {}", vault_path)))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Delegate", move |s| {
            show_delegate_confirm_dialog(s, stake_path.clone(), vote_path.clone(), vault_path.clone());
        });

    s.add_layer(dialog);
}

fn show_delegate_confirm_dialog(s: &mut Cursive, stake_path: String, vote_path: String, vault_path: String) {
    // Get stake balance
    let stake_amount = s.call_on_name("stake1_balance", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    let dialog = Dialog::new()
        .title("Confirm Delegation")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Are you sure you want to delegate?"))
                .child(DummyView.fixed_height(1))
                .child(TextView::new(format!("From: {}", stake_path)))
                .child(TextView::new(format!("To: {}", vote_path)))
                .child(TextView::new(format!("Auth: {}", vault_path)))
                .child(TextView::new(format!("Amount: {}", stake_amount)))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Confirm", move |s| {
            // Execute the command using std::process::Command
            match std::process::Command::new("solana")
                .arg("delegate-stake")
                .arg(&stake_path)
                .arg(&vote_path)
                .arg("-k")
                .arg(&vault_path)
                .output() 
            {
                Ok(output) => {
                    if output.status.success() {
                        // Command executed successfully
                        let success_msg = String::from_utf8_lossy(&output.stdout);
                        update_logs(s, &format!("Successfully delegated {} XNT", stake_amount));
                        if !success_msg.is_empty() {
                            update_logs(s, &success_msg);
                        }
                    } else {
                        // Command failed
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        update_logs(s, "Failed to delegate stake");
                        if !error_msg.is_empty() {
                            update_logs(s, &error_msg);
                        }
                    }
                }
                Err(e) => {
                    // Failed to execute command
                    update_logs(s, &format!("Error executing command: {}", e));
                }
            }

            s.pop_layer();
            s.pop_layer();  // Pop both dialogs
        });

    s.add_layer(dialog);
}

fn show_deactivate_dialog(s: &mut Cursive, stake_index: usize) {
    // Get stake key info
    let stake_path = s.call_on_name(&format!("stake{}_path_text", stake_index), |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();
    
    let stake_pubkey = s.call_on_name(&format!("stake{}_pubkey_text", stake_index), |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    // Get vault key info (stake authority)
    let vault_path = s.call_on_name("wallet_path_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();
    
    let vault_pubkey = s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
        view.get_content().source().to_string()
    }).unwrap_or_default();

    let dialog = Dialog::new()
        .title("Deactivate Stake Account")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Stake Account:"))
                .child(TextView::new(format!("Pubkey: {}", stake_pubkey)))
                .child(TextView::new(format!("Path: {}", stake_path)))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Stake Authority:"))
                .child(TextView::new(format!("Pubkey: {}", vault_pubkey)))
                .child(TextView::new(format!("Path: {}", vault_path)))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Deactivate", move |s| {
            show_deactivate_confirm_dialog(s, stake_path.clone(), vault_path.clone());
        });

    s.add_layer(dialog);
}

fn show_deactivate_confirm_dialog(s: &mut Cursive, stake_path: String, vault_path: String) {
    let dialog = Dialog::new()
        .title("Confirm Deactivation")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Are you sure you want to deactivate this stake account?"))
                .child(DummyView.fixed_height(1))
                .child(TextView::new(format!("Stake Account: {}", stake_path)))
                .child(TextView::new(format!("Authority: {}", vault_path)))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Confirm", move |s| {
            // Execute the deactivate command
            match std::process::Command::new("solana")
                .arg("deactivate-stake")
                .arg(&stake_path)
                .arg("-k")
                .arg(&vault_path)
                .output() 
            {
                Ok(output) => {
                    if output.status.success() {
                        // Command executed successfully
                        let success_msg = String::from_utf8_lossy(&output.stdout);
                        update_logs(s, "Successfully deactivated stake account");
                        if !success_msg.is_empty() {
                            update_logs(s, &success_msg);
                        }
                    } else {
                        // Command failed
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        update_logs(s, "Failed to deactivate stake account");
                        if !error_msg.is_empty() {
                            update_logs(s, &error_msg);
                        }
                    }
                }
                Err(e) => {
                    // Failed to execute command
                    update_logs(s, &format!("Error executing command: {}", e));
                }
            }

            s.pop_layer();
            s.pop_layer();  // Pop both dialogs
        });

    s.add_layer(dialog);
}

// Add helper functions for identity key operations
fn show_identity_info(s: &mut Cursive) {
    // Get current validator value
    let validator = s.call_on_name("validator_button", |button: &mut Button| {
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

    let exe_path = std::env::current_exe().unwrap_or_default();
    let exe_dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let dir_path = exe_dir.join("ledger-wallet").join(format!("validator-{}", validator));
    
    let identity_path = dir_path.join("identity.json");
    let encrypted_identity_path = dir_path.join("identity-encrypted.json");

    if identity_path.exists() {
        // For unencrypted identity file, use show_pubkey directly
        show_pubkey(s, "identity_path_text", "identity_pubkey_text", "identity_balance");
    } else if encrypted_identity_path.exists() {
        // Handle encrypted identity file
        s.add_layer(
            Dialog::new()
                .title("Enter Password")
                .content(
                    LinearLayout::vertical()
                        .child(TextView::new("Enter password to decrypt your identity key:"))
                        .child(DummyView.fixed_height(1))
                        .child(EditView::new()
                            .secret()
                            .with_name("decrypt_password")
                            .fixed_width(50))
                )
                .button("Cancel", |s| { s.pop_layer(); })
                .button("Decrypt", move |s| {
                    let password = s.call_on_name("decrypt_password", |view: &mut EditView| {
                        view.get_content()
                    }).unwrap_or_default();

                    // Read encrypted file
                    match std::fs::read(&encrypted_identity_path) {
                        Ok(encrypted_data) => {
                            let encryptor = Encryptor::new();
                            match encryptor.decrypt(password.as_bytes(), &encrypted_data) {
                                Ok(decrypted_data) => {
                                    // Get pubkey using solana address command with stdin
                                    let pubkey = match std::process::Command::new("solana")
                                        .args(["address", "-k", "-"])
                                        .stdin(std::process::Stdio::piped())
                                        .stdout(std::process::Stdio::piped())
                                        .spawn() 
                                    {
                                        Ok(mut child) => {
                                            if let Some(mut stdin) = child.stdin.take() {
                                                use std::io::Write;
                                                if let Err(e) = stdin.write_all(&decrypted_data) {
                                                    update_logs(s, &format!("Failed to write to stdin: {}", e));
                                                    s.pop_layer();
                                                    return;
                                                }
                                            }
                                            
                                            match child.wait_with_output() {
                                                Ok(output) if output.status.success() => {
                                                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                                                }
                                                _ => {
                                                    update_logs(s, "Failed to get pubkey");
                                                    s.pop_layer();
                                                    return;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            update_logs(s, &format!("Failed to spawn solana command: {}", e));
                                            s.pop_layer();
                                            return;
                                        }
                                    };

                                    // Get balance using solana balance command with stdin
                                    let balance = match std::process::Command::new("solana")
                                        .args(["balance", "--keypair", "-"])
                                        .stdin(std::process::Stdio::piped())
                                        .stdout(std::process::Stdio::piped())
                                        .spawn() 
                                    {
                                        Ok(mut child) => {
                                            if let Some(mut stdin) = child.stdin.take() {
                                                use std::io::Write;
                                                if let Err(e) = stdin.write_all(&decrypted_data) {
                                                    update_logs(s, &format!("Failed to write to stdin: {}", e));
                                                    s.pop_layer();
                                                    return;
                                                }
                                            }
                                            
                                            match child.wait_with_output() {
                                                Ok(output) if output.status.success() => {
                                                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                                                }
                                                _ => {
                                                    update_logs(s, "Failed to get balance");
                                                    s.pop_layer();
                                                    return;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            update_logs(s, &format!("Failed to spawn solana command: {}", e));
                                            s.pop_layer();
                                            return;
                                        }
                                    };

                                    // Pop password dialog
                                    s.pop_layer();

                                    // Show info dialog
                                    s.add_layer(
                                        Dialog::new()
                                            .title("Identity Account Info")
                                            .content(
                                                LinearLayout::vertical()
                                                    .child(TextView::new(format!("Public Key: {}", pubkey)))
                                                    .child(DummyView.fixed_height(1))
                                                    .child(TextView::new(format!("Balance: {} SOL", balance)))
                                            )
                                            .button("Close", |s| { s.pop_layer(); })
                                    );
                                }
                                Err(e) => {
                                    update_logs(s, &format!("Failed to decrypt: {}", e));
                                    s.pop_layer();
                                }
                            }
                        }
                        Err(e) => {
                            update_logs(s, &format!("Failed to read encrypted file: {}", e));
                            s.pop_layer();
                        }
                    }
                })
        );
    } else {
        s.add_layer(Dialog::info("No identity key found"));
    }
}

fn copy_identity_pubkey(s: &mut Cursive) {
    // Similar to copy_wallet_pubkey but for identity key
    update_logs(s, "Copying identity pubkey to clipboard...");
    // TODO: Implement actual functionality
}

fn show_identity_transfer_dialog(s: &mut Cursive) {
    // Similar to show_transfer_dialog but for identity key
    let dialog = Dialog::new()
        .title("Transfer XNT from Identity Account")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Recipient Address:"))
                .child(EditView::new().with_name("recipient_address"))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Amount (XNT):"))
                .child(EditView::new().with_name("transfer_amount"))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Transfer", |s| {
            // TODO: Implement actual transfer functionality
            update_logs(s, "Processing transfer from identity account...");
            s.pop_layer();
        });
    
    s.add_layer(dialog);
}

fn show_create_identity_dialog(s: &mut Cursive) {
    let mut protection_group = RadioGroup::new();

    let dialog = Dialog::new()
        .title("Create Identity Account")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Select protection type:"))
                .child(DummyView.fixed_height(1))
                .child(
                    LinearLayout::vertical()
                        .child(protection_group.button("without_password", "Without Password"))
                        .child(protection_group.button("with_password", "Protected by a Password (Coming Soon)"))
                )
                .child(DummyView.fixed_height(1))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Create", move |s| {
            match protection_group.selection().as_ref() {
                &"without_password" => {  // Added & to match &&str
                    // Get current validator value from validator_button
                    let validator = s.call_on_name("validator_button", |button: &mut Button| {
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
                    
                    // Get executable path
                    let exe_path = std::env::current_exe().unwrap_or_default();
                    let exe_dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new(""));
                    
                    // Create directory path relative to executable
                    let dir_path = exe_dir.join("ledger-wallet").join(format!("validator-{}", validator));
                    
                    if let Err(e) = std::fs::create_dir_all(&dir_path) {
                        update_logs(s, &format!("Failed to create directory: {}", e));
                        s.pop_layer();
                        return;
                    }

                    let output_path = dir_path.join("identity.json");
                    match std::process::Command::new("solana-keygen")
                        .args([
                            "new",
                            "--no-passphrase",
                            "-o",
                            output_path.to_str().unwrap_or_default(),
                        ])
                        .output()
                    {
                        Ok(output) => {
                            if output.status.success() {
                                update_logs(s, &format!(
                                    "Successfully created identity.json in {}",
                                    dir_path.display()
                                ));
                                
                                // Get pubkey
                                let pubkey = match std::process::Command::new("solana-keygen")
                                    .args([
                                        "pubkey",
                                        output_path.to_str().unwrap_or_default(),
                                    ])
                                    .output()
                                {
                                    Ok(pubkey_output) if pubkey_output.status.success() => {
                                        String::from_utf8_lossy(&pubkey_output.stdout).trim().to_string()
                                    }
                                    _ => "Failed to get pubkey".to_string(),
                                };

                                // Debug: Print full output
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                update_logs(s, "Debug - STDOUT:");
                                update_logs(s, &stdout);
                                update_logs(s, "Debug - STDERR:");
                                update_logs(s, &stderr);

                                // Extract seed phrase from the output
                                let mnemonic = stdout  // Changed from stderr to stdout
                                    .lines()
                                    .skip_while(|line| !line.contains("Save this seed phrase"))
                                    .skip(1)  // Skip the "Save this seed phrase" line
                                    .next()   // Get the next line (the seed phrase)
                                    .map(|line| line.trim())
                                    .unwrap_or("Failed to get recovery phrase")
                                    .to_string();

                                // First pop the creation dialog
                                s.pop_layer();

                                let pubkey_for_close = pubkey.clone();  // Clone for the close button
                                let mnemonic_for_copy = mnemonic.clone();  // Clone for the copy button
                                let success_dialog = Dialog::new()
                                    .title("Identity Account Created Successfully")
                                    .content(
                                        LinearLayout::vertical()
                                            .child(TextView::new("Your identity account has been created."))
                                            .child(DummyView.fixed_height(1))
                                            .child(TextView::new(format!("Public Key: {}", pubkey)))
                                            .child(DummyView.fixed_height(1))
                                            .child(TextView::new("Recovery Phrase (write this down and store in a safe place):"))
                                            .child(DummyView.fixed_height(1))
                                            .child(TextView::new(&mnemonic)
                                                .style(ColorStyle::title_primary())
                                                .center()
                                                .fixed_width(70))
                                            .child(DummyView.fixed_height(1))
                                    )
                                    .button("Copy Recovery Phrase", move |s| {
                                        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                                        if let Err(e) = ctx.set_contents(mnemonic_for_copy.clone()) {
                                            update_logs(s, &format!("Failed to copy to clipboard: {}", e));
                                        } else {
                                            update_logs(s, "Recovery phrase copied to clipboard");
                                        }
                                    })
                                    .button("I Have Backed Up, Close", move |s| {
                                        // Update the identity pubkey display
                                        s.call_on_name("identity_pubkey_text", |view: &mut TextView| {
                                            view.set_content(pubkey_for_close.clone());
                                        });
                                        s.pop_layer();
                                    });
                                
                                s.add_layer(success_dialog);
                                update_logs(s, &format!("Generated pubkey: {}", pubkey));
                                
                                // Debug: Print the mnemonic we're using
                                update_logs(s, &format!("Debug - Using mnemonic: {}", mnemonic));

                            } else {
                                let error = String::from_utf8_lossy(&output.stderr);
                                update_logs(s, &format!("Failed to create identity.json: {}", error));
                                s.pop_layer();
                            }
                        }
                        Err(e) => {
                            update_logs(s, &format!("Failed to execute solana-keygen: {}", e));
                            s.pop_layer();
                        }
                    }
                },
                &"with_password" => {  // Added & to match &&str
                    // Get current validator value
                    let validator = s.call_on_name("validator_button", |button: &mut Button| {
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

                    s.pop_layer();  // Pop the protection type dialog
                    show_password_input_dialog(s, validator);
                },
                _ => {
                    update_logs(s, "Please select a protection type");
                }
            }
        });

    s.add_layer(dialog);
}

fn show_password_input_dialog(s: &mut Cursive, validator: usize) {
    let dialog = Dialog::new()
        .title("Set Password")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Enter a password to protect your identity key:"))
                .child(DummyView.fixed_height(1))
                .child(EditView::new()
                    .secret()
                    .with_name("password")
                    .fixed_width(50))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Confirm password:"))
                .child(EditView::new()
                    .secret()
                    .with_name("password_confirm")
                    .fixed_width(50))
        )
        .button("Cancel", |s| { s.pop_layer(); })
        .button("Create", move |s| {
            let password = s.call_on_name("password", |view: &mut EditView| {
                view.get_content()
            }).unwrap_or_default();
            
            let password_confirm = s.call_on_name("password_confirm", |view: &mut EditView| {
                view.get_content()
            }).unwrap_or_default();

            if password != password_confirm {
                s.add_layer(
                    Dialog::info("Passwords do not match. Please try again.")
                );
                return;
            }

            // Get paths
            let exe_path = std::env::current_exe().unwrap_or_default();
            let exe_dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new(""));
            let dir_path = exe_dir.join("ledger-wallet").join(format!("validator-{}", validator));
            
            if let Err(e) = std::fs::create_dir_all(&dir_path) {
                update_logs(s, &format!("Failed to create directory: {}", e));
                s.pop_layer();
                return;
            }

            let temp_keypair_path = dir_path.join("temp.json");
            let encrypted_keypair_path = dir_path.join("identity-encrypted.json");

            // First create a temporary keypair
            match std::process::Command::new("solana-keygen")
                .args([
                    "new",
                    "--no-passphrase",
                    "-o",
                    temp_keypair_path.to_str().unwrap_or_default(),
                ])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        // Get pubkey and keypair data
                        let pubkey = match std::process::Command::new("solana-keygen")
                            .args([
                                "pubkey",
                                temp_keypair_path.to_str().unwrap_or_default(),
                            ])
                            .output()
                        {
                            Ok(pubkey_output) if pubkey_output.status.success() => {
                                String::from_utf8_lossy(&pubkey_output.stdout).trim().to_string()
                            }
                            _ => {
                                update_logs(s, "Failed to get pubkey");
                                s.pop_layer();
                                return;
                            }
                        };

                        // Read the keypair file
                        let keypair_data = match std::fs::read(&temp_keypair_path) {
                            Ok(data) => data,
                            Err(e) => {
                                update_logs(s, &format!("Failed to read keypair file: {}", e));
                                s.pop_layer();
                                return;
                            }
                        };

                        // Encrypt the keypair
                        let encryptor = Encryptor::new();
                        match encryptor.encrypt(password.as_bytes(), &keypair_data) {
                            Ok(encrypted_data) => {
                                // Write encrypted data to file
                                if let Err(e) = std::fs::write(&encrypted_keypair_path, &encrypted_data) {
                                    update_logs(s, &format!("Failed to write encrypted keypair: {}", e));
                                    s.pop_layer();
                                    return;
                                }
                            }
                            Err(e) => {
                                update_logs(s, &format!("Failed to encrypt keypair: {}", e));
                                s.pop_layer();
                                return;
                            }
                        }

                        // Remove temporary keypair file
                        let _ = std::fs::remove_file(&temp_keypair_path);

                        // Extract seed phrase from the output
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let mnemonic = stderr
                            .lines()
                            .skip_while(|line| !line.contains("Save this seed phrase"))
                            .skip(1)
                            .next()
                            .map(|line| line.trim())
                            .unwrap_or("Failed to get recovery phrase")
                            .to_string();

                        s.pop_layer();  // Pop password dialog

                        // Show success dialog
                        let pubkey_for_close = pubkey.clone();
                        let mnemonic_for_copy = mnemonic.clone();
                        let success_dialog = Dialog::new()
                            .title("Identity Account Created Successfully")
                            .content(
                                LinearLayout::vertical()
                                    .child(TextView::new("Your password-protected identity account has been created."))
                                    .child(DummyView.fixed_height(1))
                                    .child(TextView::new(format!("Public Key: {}", pubkey)))
                                    .child(DummyView.fixed_height(1))
                                    .child(TextView::new("Recovery Phrase (write this down and store in a safe place):"))
                                    .child(DummyView.fixed_height(1))
                                    .child(TextView::new(&mnemonic)
                                        .style(ColorStyle::title_primary())
                                        .center()
                                        .fixed_width(70))
                                    .child(DummyView.fixed_height(1))
                            )
                            .button("Copy Recovery Phrase", move |s| {
                                let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                                if let Err(e) = ctx.set_contents(mnemonic_for_copy.clone()) {
                                    update_logs(s, &format!("Failed to copy to clipboard: {}", e));
                                } else {
                                    update_logs(s, "Recovery phrase copied to clipboard");
                                }
                            })
                            .button("I Have Backed Up, Close", move |s| {
                                s.call_on_name("identity_pubkey_text", |view: &mut TextView| {
                                    view.set_content(pubkey_for_close.clone());
                                });
                                s.pop_layer();
                            });

                        s.add_layer(success_dialog);
                        update_logs(s, &format!("Generated encrypted identity keypair with pubkey: {}", pubkey));
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        update_logs(s, &format!("Failed to create keypair: {}", error));
                        s.pop_layer();
                    }
                }
                Err(e) => {
                    update_logs(s, &format!("Failed to execute solana-keygen: {}", e));
                    s.pop_layer();
                }
            }
        });

    s.add_layer(dialog);
}
