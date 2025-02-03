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

// Add a constant for maximum log lines
const MAX_LOG_LINES: usize = 100;

// Initialize regex pattern for ANSI escape codes
lazy_static! {
    static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]|\x1B\[[0-9;]*m").unwrap();
}

lazy_static! {
    static ref CURRENT_NETWORK: Mutex<String> = Mutex::new("testnet".to_string());
}

// Configuration structures
#[derive(Serialize, Deserialize)]
struct WalletConfig {
    name: String,
    vault_key: KeyConfig,
    vote_key: KeyConfig,
    stake_keys: BTreeMap<usize, KeyConfig>,
}

#[derive(Serialize, Deserialize)]
struct KeyConfig {
    x: String,
    y: String,
}

// Initialize configuration
fn init_config() -> Result<(), String> {
    // Get executable path
    let exe_path = env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?;
    
    // Get executable directory
    let exe_dir = exe_path.parent()
        .ok_or_else(|| "Failed to get executable directory".to_string())?;
    
    println!("Executable directory: {:?}", exe_dir);
    
    // Create ledger-wallet directory in the same directory as the executable
    let config_dir = exe_dir.join("ledger-wallet");
    println!("Creating config directory at: {:?}", config_dir);
    
    if !config_dir.exists() {
        fs::create_dir(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config_path = config_dir.join("ledger.json");
    if !config_path.exists() {
        // Create default configuration
        let mut validators = BTreeMap::new();
        
        // Create configuration for each validator (0-9)
        for i in 0..10 {
            let stake_keys: BTreeMap<usize, KeyConfig> = (1..=5).map(|stake_index| {
                (stake_index, KeyConfig {
                    x: i.to_string(),
                    y: stake_index.to_string(),
                })
            }).collect();

            validators.insert(format!("validator_{}", i), WalletConfig {
                name: format!("Validator {}", i),
                vault_key: KeyConfig {
                    x: i.to_string(),
                    y: "N/A".to_string(),
                },
                vote_key: KeyConfig {
                    x: i.to_string(),
                    y: "0".to_string(),
                },
                stake_keys,
            });
        }

        // Write configuration to file
        let json = serde_json::to_string_pretty(&validators)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write(&config_path, json)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    }

    Ok(())
}

// Call this function when the program starts
pub fn setup_ledger() {
    println!("Starting ledger configuration setup...");
    match init_config() {
        Ok(_) => println!("Ledger configuration initialized successfully"),
        Err(e) => println!("Failed to initialize ledger configuration: {}", e),
    }
}

// Helper function to update logs when Cursive instance is not available
fn update_logs_static(message: &str) {
    println!("{}", message);  // For now, just print to stdout
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
            }
        }
    }
}

// Add helper function to create stake key section
fn create_stake_key_section(index: usize, default_y: usize) -> LinearLayout {
    LinearLayout::vertical()
        .child(
            LinearLayout::horizontal()
                .child(TextView::new(format!("STAKE KEY {}:", index)))
                .child(DummyView.fixed_width(1))
                .child(TextView::new("").with_name(format!("stake{}_balance", index)).fixed_width(20))
                .child(DummyView.fixed_width(1))
                .child(TextView::new("").with_name(format!("stake{}_pubkey_text", index)))
        )
        .child(
            LinearLayout::horizontal()
                .child(Button::new("▼ Select x' (0)", move |s| {
                    show_stake_account_select(s, index);
                })
                .with_name(format!("stake{}_x_button", index))
                .fixed_width(20))
                .child(DummyView.fixed_width(1))
                .child(Button::new(format!("▼ Select y' ({})", default_y), move |s| {
                    show_stake_address_select(s, index);
                })
                .with_name(format!("stake{}_y_button", index))
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
        .child(
            LinearLayout::horizontal()
                .child(Button::new("Show Balance & PubKey", move |s| {
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
                        if !pubkey.is_empty() {
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
                    show_transfer_dialog(s, "stake", Some(index));
                }).fixed_width(15))
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
    
    // Add 10 account options
    for i in 0..10 {
        select.add_item(format!("Account {}", i), i.to_string());
    }
    
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
    
    let current_value = s.call_on_name(&format!("stake{}_y_button", stake_index), |button: &mut Button| {
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
        .fixed_height(2)
        .with_name("dashboard");

    let config = Panel::new(
        LinearLayout::vertical()
            .child(Button::new("Connect Ledger", connect_ledger))
            .child(DummyView.fixed_height(1))
            // Simplified validator selector
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("Select Validator: "))
                        .child(Button::new("▼ Validator (0)", show_validator_select)
                            .with_name("validator_button")
                            .fixed_width(20))
            )
            .child(DummyView.fixed_height(1))
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
            // VAULT KEY section
            .child(
                LinearLayout::vertical()
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("VAULT (ID/WITHDRAW) KEY:"))
                            .child(DummyView.fixed_width(1))
                            .child(TextView::new("").with_name("vault_balance").fixed_width(20))
                            .child(DummyView.fixed_width(1))
                            .child(TextView::new("").with_name("wallet_pubkey_text"))
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(Button::new("▼ Select x' (0)", show_account_select)
                                .with_name("x_button")
                                .fixed_width(20))
                            .child(DummyView.fixed_width(1))
                            .child(Button::new("▼ Select y' (N/A)", show_address_select)
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
                    .child(
                        LinearLayout::horizontal()
                            .child(Button::new("Show Balance & PubKey", move |s| {
                                show_pubkey(s, "wallet_path_text", "wallet_pubkey_text", "vault_balance");
                            }).fixed_width(25))
                            .child(DummyView.fixed_width(1))
                            .child(Button::new("Copy PubKey", |s| {
                                if let Some(pubkey) = s.call_on_name("wallet_pubkey_text", |view: &mut TextView| {
                                    view.get_content().source().to_string()
                                }) {
                                    if !pubkey.is_empty() {
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
                                show_transfer_dialog(s, "vault", None);
                            }).fixed_width(15))
                    )
            )
            .child(DummyView.fixed_height(1))
            // VOTE KEY section
            .child(
                LinearLayout::vertical()
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("VOTE KEY:"))
                            .child(DummyView.fixed_width(1))
                            .child(TextView::new("").with_name("vote_balance").fixed_width(20))
                            .child(DummyView.fixed_width(1))
                            .child(TextView::new("").with_name("vote_pubkey_text"))
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(Button::new("▼ Select x' (0)", show_vote_account_select)
                                .with_name("vote_x_button")
                                .fixed_width(20))
                            .child(DummyView.fixed_width(1))
                            .child(Button::new("▼ Select y' (0)", show_vote_address_select)
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
                    .child(
                        LinearLayout::horizontal()
                            .child(Button::new("Show Balance & PubKey", move |s| {
                                show_pubkey(s, "vote_path_text", "vote_pubkey_text", "vote_balance");
                            }).fixed_width(25))
                            .child(DummyView.fixed_width(1))
                            .child(Button::new("Copy PubKey", |s| {
                                if let Some(pubkey) = s.call_on_name("vote_pubkey_text", |view: &mut TextView| {
                                    view.get_content().source().to_string()
                                }) {
                                    if !pubkey.is_empty() {
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
                                show_transfer_dialog(s, "vote", None);
                            }).fixed_width(15))
                    )
            )
            .child(DummyView.fixed_height(1))
            // STAKE KEYs
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
    .title("Configuration")
    .full_width()
    .full_height();

    let logs = Panel::new(
        ScrollView::new(TextView::new(""))
            .scroll_strategy(cursive::view::ScrollStrategy::StickToBottom)
            .with_name("logs")
    )
    .title("Logs")
    .full_width()
    .fixed_height(10);

    LinearLayout::vertical()
        .child(dashboard)
        .child(config)
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

        // Log the change
        update_logs(s, &format!("Switched to Validator {}", validator));

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

    let dialog = Dialog::new()
        .title(format!("Transfer XNT from {}", title))
        .content(
            LinearLayout::vertical()
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

    let dialog = Dialog::new()
        .title("Confirm Transfer")
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Please confirm the transfer details:"))
                .child(DummyView.fixed_height(1))
                .child(TextView::new(format!("From: {}", from_path)))
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
        .arg("--from")
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
