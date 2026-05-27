use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    components: Vec<String>,
}

impl Sentence {
    pub fn new(components: Vec<String>) -> Self {
        Sentence { components }
    }

    pub fn from_strs(components: &[&str]) -> Self {
        Sentence {
            components: components.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn as_slice(&self) -> &[String] {
        &self.components
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.components.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = String> {
        self.components.into_iter()
    }
}

impl fmt::Display for Sentence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.components.join("."))
    }
}

impl From<Vec<String>> for Sentence {
    fn from(components: Vec<String>) -> Self {
        Sentence { components }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Number(f64),
    Boolean(bool),
    Variant(String),
    String(String),
    ActorRef(String),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Constant::Number(n) => n.to_string(),
            Constant::Boolean(b) => b.to_string(),
            Constant::Variant(v) => v.clone(),
            Constant::String(s) => format!("\"{}\"", s),
            Constant::ActorRef(r) => format!("@{}", r),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Variant(String),
    String(String),
    ActorRef(String),
    Variable(Sentence),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Variant(v) => v.clone(),
            Value::String(s) => format!("\"{}\"", s),
            Value::ActorRef(r) => format!("@{}", r),
            Value::Variable(sentence) => sentence.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl From<&Constant> for Value {
    fn from(constant: &Constant) -> Self {
        match constant {
            Constant::Number(n) => Value::Number(*n),
            Constant::Boolean(b) => Value::Boolean(*b),
            Constant::Variant(v) => Value::Variant(v.clone()),
            Constant::String(s) => Value::String(s.clone()),
            Constant::ActorRef(r) => Value::ActorRef(r.clone()),
        }
    }
}
