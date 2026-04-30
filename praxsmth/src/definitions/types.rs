use crate::definitions::{PraxsmthValue, Serialize};

use crate::definitions::{FieldTypes, Sentence};

#[derive(Debug, Clone)]
pub struct PraxsmthType {
    pub name: String,
    pub fields: FieldTypes,
    pub data: PraxsmthTypeData,
}

#[derive(Debug, Clone)]
pub enum PraxsmthTypeData {
    Trait,
    Directional {
        complement: String,
    },
    Reciprocal,
    Evaluation {
        complement: String,
    },
    Emotion,
    Practice {
        params: Vec<String>,
        display: Option<String>,
        actions: Vec<PracticeAction>,
    },
}

impl Serialize for PraxsmthType {
    fn serialize(&self) -> String {
        match &self.data {
            PraxsmthTypeData::Trait => {
                format!("trait {} {{{}}}", self.name, self.fields.serialize())
            }
            PraxsmthTypeData::Directional { complement } => format!(
                "directional {}/{} {{{}}}",
                self.name,
                complement,
                self.fields.serialize()
            ),
            PraxsmthTypeData::Reciprocal => {
                format!("reciprocal {} {{{}}}", self.name, self.fields.serialize())
            }
            PraxsmthTypeData::Evaluation { complement } => {
                format!(
                    "evaluation {}/{} {{{}}}",
                    self.name,
                    complement,
                    self.fields.serialize()
                )
            }
            PraxsmthTypeData::Emotion => {
                format!("emotion {} {{{}}}", self.name, self.fields.serialize())
            }
            PraxsmthTypeData::Practice {
                params, display, ..
            } => {
                let params_str = params.join(", ");
                let display_str = display
                    .as_ref()
                    .map(|d| format!(" display \"{}\"", d))
                    .unwrap_or_default();
                format!(
                    "practice {}({}){} {{{}}}",
                    self.name,
                    params_str,
                    display_str,
                    self.fields.serialize()
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PracticeAction {
    pub for_actor: String,
    pub name: String,
    pub conditions: Vec<PracticeCondition>,
    pub outcomes: Vec<PracticeOutcome>,
}

#[derive(Debug, Clone)]
pub enum PracticeCondition {
    Value(PraxsmthValue),
    And(Box<PracticeCondition>, Box<PracticeCondition>),
    Or(Box<PracticeCondition>, Box<PracticeCondition>),
    Is(Box<PracticeCondition>, Box<PracticeCondition>),
    Not(Box<PracticeCondition>),
}

#[derive(Debug, Clone)]
pub enum PracticeOutcome {
    Print(String),
    Delete(Sentence),
    Set(Sentence, PraxsmthValue),
    Increase(Sentence, i64),
    Cycle(Sentence, i64),
}
