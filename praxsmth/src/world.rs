use crate::Serialize;
use std::collections::HashMap;

pub enum PraxsmthWorldValues {
    Agent(Agent),
}

pub struct Agent {
    pub name: String,
    pub subagents: HashMap<String, Agent>,
}

impl Serialize for Agent {
    fn serialize(&self) -> String {
        if self.subagents.is_empty() {
            self.name.clone()
        } else {
            let subagents_str: Vec<_> = self.subagents.iter().map(|(_, a)| a.serialize()).collect();
            format!("agent {} {{{}}}", self.name, subagents_str.join(", "))
        }
    }
}
