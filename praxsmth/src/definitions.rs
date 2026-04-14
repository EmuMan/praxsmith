use std::collections::HashMap;

pub mod types;
pub mod world;

pub trait Serialize {
    fn serialize(&self) -> String;
}

pub type Sentence = Vec<String>;

impl Serialize for Sentence {
    fn serialize(&self) -> String {
        self.join(".")
    }
}

#[derive(Debug, Clone)]
pub enum PraxsmthConstant {
    Number(i64),
    Variant(String),
    String(String),
}

impl Serialize for PraxsmthConstant {
    fn serialize(&self) -> String {
        match self {
            PraxsmthConstant::Number(n) => n.to_string(),
            PraxsmthConstant::Variant(v) => v.clone(),
            PraxsmthConstant::String(s) => format!("\"{}\"", s),
        }
    }
}

pub enum PraxsmthValue {
    Number(i64),
    Variant(String),
    String(String),
    Variable(Sentence),
}

impl Serialize for PraxsmthValue {
    fn serialize(&self) -> String {
        match self {
            PraxsmthValue::Number(n) => n.to_string(),
            PraxsmthValue::Variant(v) => v.clone(),
            PraxsmthValue::String(s) => format!("\"{}\"", s),
            PraxsmthValue::Variable(s) => s.join("."),
        }
    }
}

pub type TypeFields = HashMap<String, PraxsmthField>;

#[derive(Debug, Clone)]
pub enum PraxsmthField {
    NumberRange(i64, i64),
    VariantList(Vec<String>),
}

impl Serialize for PraxsmthField {
    fn serialize(&self) -> String {
        match self {
            PraxsmthField::NumberRange(start, end) => format!("{}..{}", start, end),
            PraxsmthField::VariantList(variants) => variants.join(" | "),
        }
    }
}

impl Serialize for TypeFields {
    fn serialize(&self) -> String {
        let fields_str: Vec<_> = self
            .iter()
            .map(|(name, field)| format!("{}: {}", name, field.serialize()))
            .collect();
        fields_str.join(", ")
    }
}
