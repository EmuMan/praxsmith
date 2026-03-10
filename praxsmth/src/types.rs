use crate::Serialize;

pub enum PraxsmthTypes {
    Trait(Trait),
    Directional(Directional),
    Reciprocal(Reciprocal),
    Evaluation(Evaluation),
    Emotion(Emotion),
    Practice(Practice),
}

impl Serialize for PraxsmthTypes {
    fn serialize(&self) -> String {
        match self {
            PraxsmthTypes::Trait(t) => t.serialize(),
            PraxsmthTypes::Directional(d) => d.serialize(),
            PraxsmthTypes::Reciprocal(r) => r.serialize(),
            PraxsmthTypes::Evaluation(e) => e.serialize(),
            PraxsmthTypes::Emotion(em) => em.serialize(),
            PraxsmthTypes::Practice(p) => p.serialize(),
        }
    }
}

pub enum Field {
    NumberRange(i64, i64),
    VariantList(Vec<String>),
}

impl Serialize for Field {
    fn serialize(&self) -> String {
        match self {
            Field::NumberRange(start, end) => format!("{}..{}", start, end),
            Field::VariantList(variants) => variants.join(" | "),
        }
    }
}

impl Serialize for Vec<(String, Field)> {
    fn serialize(&self) -> String {
        let fields_str: Vec<_> = self
            .iter()
            .map(|(name, field)| format!("{}: {}", name, field.serialize()))
            .collect();
        fields_str.join(", ")
    }
}

pub struct Trait {
    pub name: String,
    pub fields: Vec<(String, Field)>,
}

impl Serialize for Trait {
    fn serialize(&self) -> String {
        if self.fields.is_empty() {
            self.name.clone()
        } else {
            format!("trait {} {{{}}}", self.name, self.fields.serialize())
        }
    }
}

pub struct Directional {
    pub forward_name: String,
    pub backward_name: String,
    pub fields: Vec<(String, Field)>,
}

impl Serialize for Directional {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!(
            "directional {} / {} {{{}}}",
            self.forward_name, self.backward_name, fields_str
        )
    }
}

pub struct Reciprocal {
    pub name: String,
    pub fields: Vec<(String, Field)>,
}

impl Serialize for Reciprocal {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!("reciprocal {} {{{}}}", self.name, fields_str)
    }
}

pub struct Evaluation {
    pub forward_name: String,
    pub backward_name: String,
    pub fields: Vec<(String, Field)>,
}

impl Serialize for Evaluation {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!(
            "evaluation {} / {} {{{}}}",
            self.forward_name, self.backward_name, fields_str
        )
    }
}

pub struct Emotion {
    pub name: String,
    pub fields: Vec<(String, Field)>,
}

impl Serialize for Emotion {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!("emotion {} {{{}}}", self.name, fields_str)
    }
}

pub struct Practice {
    pub name: String,
    pub params: Vec<String>,
    pub display: Option<String>,
    pub actions: Option<Vec<PracticeAction>>,
    pub fields: Vec<(String, Field)>,
}

pub struct PracticeAction {
    pub for_actor: String,
    pub name: String,
    pub conditions: Vec<PracticeCondition>,
    pub outcomes: Vec<PracticeOutcome>,
}

pub enum PracticeCondition {
    Sentence(String),
    And(Box<PracticeCondition>, Box<PracticeCondition>),
    Or(Box<PracticeCondition>, Box<PracticeCondition>),
    Is(Box<PracticeCondition>, Box<PracticeCondition>),
    Not(Box<PracticeCondition>),
}

pub enum PracticeOutcome {
    Print(String),
    Delete(String),
    Set(String, String),
    Increase(String, i64),
    Cycle(String, i64),
}

impl Serialize for Practice {
    fn serialize(&self) -> String {
        let params_str = self.params.join(", ");
        let display_str = self
            .display
            .as_ref()
            .map(|d| format!(" display \"{}\"", d))
            .unwrap_or_default();
        let fields_str = self.fields.serialize();
        format!(
            "practice {}({}){} {{{}}}",
            self.name, params_str, display_str, fields_str
        )
    }
}
