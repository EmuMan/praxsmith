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
    pub conditions: Vec<Expression>,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Value(PraxsmthValue),
    /// Boolean, Boolean -> Boolean
    And(Box<Expression>, Box<Expression>),
    /// Boolean, Boolean -> Boolean
    Or(Box<Expression>, Box<Expression>),
    /// T, T -> Boolean
    Is(Box<Expression>, Box<Expression>),
    /// Boolean -> Boolean
    Not(Box<Expression>),
    /// Boolean... -> Boolean (`for all X, Y` = Y must hold for every binding of X)
    ForAll(String, Box<Expression>),
    /// Boolean... -> Boolean (`any X where Y` = there exists some binding of X for which Y holds)
    Any(String, Box<Expression>),
    /// Number (`count SYM where FILTER` = how many bindings of SYM satisfy FILTER)
    Count(String, Box<Expression>),
    /// Number (`OP BODY across SYM where FILTER` = reduce BODY over the bindings
    /// of SYM that satisfy FILTER). With no matching bindings, evaluates to 0.
    Aggregate {
        op: AggregateOp,
        /// Numeric expression evaluated once per matching binding of `var`.
        body: Box<Expression>,
        /// The bound variable iterated over.
        var: String,
        /// Boolean expression selecting which bindings of `var` contribute.
        filter: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateOp {
    Sum,
    Average,
    Min,
    Max,
}

#[derive(Debug, Clone)]
pub enum Effect {
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
