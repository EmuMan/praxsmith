use crate::definitions::{PraxsmthValue, Serialize};

use crate::definitions::{Sentence, TypeFields};

pub enum PraxsmthType {
    Trait(TraitType),
    Directional(DirectionalType),
    Reciprocal(ReciprocalType),
    Evaluation(EvaluationType),
    Emotion(EmotionType),
    Practice(PracticeType),
}

impl Serialize for PraxsmthType {
    fn serialize(&self) -> String {
        match self {
            PraxsmthType::Trait(t) => t.serialize(),
            PraxsmthType::Directional(d) => d.serialize(),
            PraxsmthType::Reciprocal(r) => r.serialize(),
            PraxsmthType::Evaluation(e) => e.serialize(),
            PraxsmthType::Emotion(em) => em.serialize(),
            PraxsmthType::Practice(p) => p.serialize(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraitType {
    pub name: String,
    pub fields: TypeFields,
}

impl Serialize for TraitType {
    fn serialize(&self) -> String {
        if self.fields.is_empty() {
            self.name.clone()
        } else {
            format!("trait {} {{{}}}", self.name, self.fields.serialize())
        }
    }
}

pub struct DirectionalType {
    pub forward_name: String,
    pub backward_name: String,
    pub fields: TypeFields,
}

impl Serialize for DirectionalType {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!(
            "directional {} / {} {{{}}}",
            self.forward_name, self.backward_name, fields_str
        )
    }
}

pub struct ReciprocalType {
    pub name: String,
    pub fields: TypeFields,
}

impl Serialize for ReciprocalType {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!("reciprocal {} {{{}}}", self.name, fields_str)
    }
}

pub struct EvaluationType {
    pub forward_name: String,
    pub backward_name: String,
    pub fields: TypeFields,
}

impl Serialize for EvaluationType {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!(
            "evaluation {} / {} {{{}}}",
            self.forward_name, self.backward_name, fields_str
        )
    }
}

pub struct EmotionType {
    pub name: String,
    pub fields: TypeFields,
}

impl Serialize for EmotionType {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!("emotion {} {{{}}}", self.name, fields_str)
    }
}

pub struct PracticeType {
    pub name: String,
    pub params: Vec<String>,
    pub display: Option<String>,
    pub actions: Option<Vec<PracticeAction>>,
    pub fields: TypeFields,
}

pub struct PracticeAction {
    pub for_actor: String,
    pub name: String,
    pub conditions: Vec<PracticeCondition>,
    pub outcomes: Vec<PracticeOutcome>,
}

pub enum PracticeCondition {
    Value(PraxsmthValue),
    And(Box<PracticeCondition>, Box<PracticeCondition>),
    Or(Box<PracticeCondition>, Box<PracticeCondition>),
    Is(Box<PracticeCondition>, Box<PracticeCondition>),
    Not(Box<PracticeCondition>),
}

pub enum PracticeOutcome {
    Print(String),
    Delete(Sentence),
    Set(Sentence, PraxsmthValue),
    Increase(Sentence, i64),
    Cycle(Sentence, i64),
}

impl Serialize for PracticeType {
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
