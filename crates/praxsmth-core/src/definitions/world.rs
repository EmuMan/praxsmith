use crate::definitions::{Sentence, Serialize};
use std::collections::HashMap;

use crate::definitions::PraxsmthConstant;

#[derive(Debug, Clone)]
pub enum PraxsmthWorldDefinition {
    AgentInfo(AgentInfo),
    Declaration(Declaration),
}

impl Serialize for PraxsmthWorldDefinition {
    fn serialize(&self) -> String {
        match self {
            PraxsmthWorldDefinition::AgentInfo(a) => a.serialize(),
            PraxsmthWorldDefinition::Declaration(d) => d.serialize(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub subagents: HashMap<String, AgentInfo>,
}

impl Serialize for AgentInfo {
    fn serialize(&self) -> String {
        if self.subagents.is_empty() {
            self.name.clone()
        } else {
            let subagents_str: Vec<_> = self.subagents.iter().map(|(_, a)| a.serialize()).collect();
            format!("{} {{{}}}", self.name, subagents_str.join(", "))
        }
    }
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub sentence: Sentence,
    pub fields: HashMap<String, PraxsmthConstant>,
}

impl Serialize for Declaration {
    fn serialize(&self) -> String {
        let fields_str: Vec<_> = self
            .fields
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value.serialize()))
            .collect();
        format!(
            "declaration {} {{{}}}",
            self.sentence.serialize(),
            fields_str.join(", ")
        )
    }
}
