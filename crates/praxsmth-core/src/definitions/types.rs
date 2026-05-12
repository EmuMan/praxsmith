use crate::definitions::world::Declaration;
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
        exclusive: bool,
    },
    Reciprocal,
    Evaluation {
        complement: String,
    },
    Emotion,
    Practice {
        params: Vec<String>,
        actions: Vec<PracticeAction>,
    },
}

impl Serialize for PraxsmthType {
    fn serialize(&self) -> String {
        match &self.data {
            PraxsmthTypeData::Trait => {
                format!("trait {} {{{}}}", self.name, self.fields.serialize())
            }
            PraxsmthTypeData::Directional {
                complement,
                exclusive,
            } => format!(
                "{}directional {}/{} {{{}}}",
                if *exclusive { "exclusive " } else { "" },
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
            PraxsmthTypeData::Practice { params, .. } => {
                let params_str = params.join(", ");
                format!(
                    "practice {}({}) {{{}}}",
                    self.name,
                    params_str,
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
    pub conditions: Vec<Condition>,
    pub outcomes: Vec<PracticeOutcome>,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Value(PraxsmthValue),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Is(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

#[derive(Debug, Clone)]
pub enum PracticeOutcome {
    Broadcast(String),
    Say(String),
    Activate(String),
    Deactivate(String),
    Delete(Sentence),
    Set(Declaration),
    Update(Sentence, PraxsmthValue),
    Increase(Sentence, i64),
    Cycle(Sentence, i64),
}
