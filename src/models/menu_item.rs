use super::tab_page::TabPage;

pub struct MenuItem {
    pub name: String,
    pub tabs: Vec<TabPage>,
    pub active_tab: usize,
}

impl MenuItem {
    pub fn new(name: String, tabs: Vec<TabPage>) -> Self {
        Self {
            name,
            tabs,
            active_tab: 0,
        }
    }
}