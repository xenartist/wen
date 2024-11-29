use crate::models::MenuItem;
use crate::wallets::{ledger_wallet, software_wallet, solana_cli};

pub struct App {
    pub menu_items: Vec<MenuItem>,
    pub selected_menu: usize,
}

impl App {
    pub fn new() -> Self {
        let menu_items = vec![
            solana_cli::create_cli_menu(),
            ledger_wallet::create_ledger_menu(),
            software_wallet::create_software_menu(),
        ];

        Self {
            menu_items,
            selected_menu: 0,
        }
    }

    pub fn handle_menu_click(&mut self, row: u16) -> bool {
        if let Some(index) = row.checked_sub(1) {
            if (index as usize) < self.menu_items.len() {
                self.selected_menu = index as usize;
                return true;
            }
        }
        false
    }

    pub fn current_menu_item(&self) -> &MenuItem {
        &self.menu_items[self.selected_menu]
    }

    pub fn current_menu_item_mut(&mut self) -> &mut MenuItem {
        &mut self.menu_items[self.selected_menu]
    }

    pub fn handle_tab_click(&mut self, relative_column: u16, tab_width: u16) -> bool {
        let clicked_tab = relative_column / tab_width;
        if (clicked_tab as usize) < self.current_menu_item().tabs.len() {
            self.current_menu_item_mut().active_tab = clicked_tab as usize;
            true
        } else {
            false
        }
    }
}