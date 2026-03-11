use crate::definitions::Serialize;
use std::collections::HashMap;

use crate::definitions::PraxsmthConstant;

pub enum PraxsmthWorldDefinition {
    Agent(Agent),
    Declaration(Declaration),
}

impl Serialize for PraxsmthWorldDefinition {
    fn serialize(&self) -> String {
        match self {
            PraxsmthWorldDefinition::Agent(a) => a.serialize(),
            PraxsmthWorldDefinition::Declaration(d) => d.serialize(),
        }
    }
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
            format!("{} {{{}}}", self.name, subagents_str.join(", "))
        }
    }
}

pub struct Declaration {
    pub name: String,
    pub fields: HashMap<String, PraxsmthConstant>,
}

impl Serialize for Declaration {
    fn serialize(&self) -> String {
        let fields_str: Vec<_> = self
            .fields
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value.serialize()))
            .collect();
        format!("declaration {} {{{}}}", self.name, fields_str.join(", "))
    }
}
