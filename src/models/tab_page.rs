#[derive(Clone)]
pub struct TabPage {
    pub name: String,
    pub config: Vec<String>,
    pub logs: Vec<String>,
    pub scroll: u16,
}

impl TabPage {
    pub fn new(name: String) -> Self {
        Self {
            name,
            config: Vec::new(),
            logs: Vec::new(),
            scroll: 0,
        }
    }

    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
        self.scroll = self.logs.len() as u16;
    }
}