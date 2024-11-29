use crate::models::{MenuItem, TabPage};
use std::process::Command;

pub fn create_cli_menu() -> MenuItem {
    let mut tab = TabPage::new("Install CLI".to_string());
    
    // config
    tab.config.push("Solana CLI Installation Guide:".to_string());
    tab.config.push("1. sh -c \"$(curl -sSfL https://release.solana.com/v1.17.0/install)\"".to_string());
    tab.config.push("2. export PATH=\"/root/.local/share/solana/install/active_release/bin:$PATH\"".to_string());
    tab.config.push("3. solana --version".to_string());
    
    let tabs = vec![tab];
    MenuItem::new("Solana CLI".to_string(), tabs)
}

pub fn handle_install_cli_tab(tab: &mut TabPage) {
    // check if solana cli is installed
    match Command::new("solana").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                tab.add_log(format!("Solana CLI already installed: {}", version));
            } else {
                tab.add_log("Solana CLI not found. Please follow the installation guide above.".to_string());
            }
        }
        Err(_) => {
            tab.add_log("Solana CLI not found. Please follow the installation guide above.".to_string());
        }
    }
}