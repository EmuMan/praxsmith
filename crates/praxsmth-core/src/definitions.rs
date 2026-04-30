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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PraxsmthConstant {
    Number(i64),
    Boolean(bool),
    Variant(String),
    String(String),
}

impl Serialize for PraxsmthConstant {
    fn serialize(&self) -> String {
        match self {
            PraxsmthConstant::Number(n) => n.to_string(),
            PraxsmthConstant::Boolean(b) => b.to_string(),
            PraxsmthConstant::Variant(v) => v.clone(),
            PraxsmthConstant::String(s) => format!("\"{}\"", s),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PraxsmthValue {
    Number(i64),
    Boolean(bool),
    Variant(String),
    String(String),
    Variable(Sentence),
}

impl Serialize for PraxsmthValue {
    fn serialize(&self) -> String {
        match self {
            PraxsmthValue::Number(n) => n.to_string(),
            PraxsmthValue::Boolean(b) => b.to_string(),
            PraxsmthValue::Variant(v) => v.clone(),
            PraxsmthValue::String(s) => format!("\"{}\"", s),
            PraxsmthValue::Variable(s) => s.join("."),
        }
    }
}

impl From<&PraxsmthConstant> for PraxsmthValue {
    fn from(constant: &PraxsmthConstant) -> Self {
        match constant {
            PraxsmthConstant::Number(n) => PraxsmthValue::Number(*n),
            PraxsmthConstant::Boolean(b) => PraxsmthValue::Boolean(*b),
            PraxsmthConstant::Variant(v) => PraxsmthValue::Variant(v.clone()),
            PraxsmthConstant::String(s) => PraxsmthValue::String(s.clone()),
        }
    }
}

pub type FieldTypes = HashMap<String, PraxsmthField>;

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

impl Serialize for FieldTypes {
    fn serialize(&self) -> String {
        let fields_str: Vec<_> = self
            .iter()
            .map(|(name, field)| format!("{}: {}", name, field.serialize()))
            .collect();
        fields_str.join(", ")
    }
}
