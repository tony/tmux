use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Flag(bool),
}

pub struct Options {
    values: HashMap<String, OptionValue>,
}

impl Options {
    pub fn get(&self, name: &str) -> Option<&OptionValue> {
        todo!()
    }
    
    pub fn set(&mut self, name: &str, val: OptionValue) {
        todo!()
    }
}
--- END FILE ---
