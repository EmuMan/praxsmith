use std::fs;

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "praxsmth.pest"]
struct PraxsmthParser;

pub trait Serialize {
    fn serialize(&self) -> String;
}

pub enum PraxsmthValue {
    Agent(Agent),
    Trait(Trait),
    Directional(Directional),
    Reciprocal(Reciprocal),
    Evaluation(Evaluation),
    Emotion(Emotion),
    Practice(Practice),
}

impl Serialize for PraxsmthValue {
    fn serialize(&self) -> String {
        match self {
            PraxsmthValue::Agent(a) => a.serialize(),
            PraxsmthValue::Trait(t) => t.serialize(),
            PraxsmthValue::Directional(d) => d.serialize(),
            PraxsmthValue::Reciprocal(r) => r.serialize(),
            PraxsmthValue::Evaluation(e) => e.serialize(),
            PraxsmthValue::Emotion(em) => em.serialize(),
            PraxsmthValue::Practice(p) => p.serialize(),
        }
    }
}

pub struct Agent {
    name: String,
    subagents: Vec<Agent>,
}

impl Serialize for Agent {
    fn serialize(&self) -> String {
        if self.subagents.is_empty() {
            self.name.clone()
        } else {
            let subagents_str: Vec<_> = self.subagents.iter().map(|a| a.serialize()).collect();
            format!("agent {} {{{}}}", self.name, subagents_str.join(", "))
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
    name: String,
    fields: Vec<(String, Field)>,
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
    forward_name: String,
    backward_name: String,
    fields: Vec<(String, Field)>,
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
    name: String,
    fields: Vec<(String, Field)>,
}

impl Serialize for Reciprocal {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!("reciprocal {} {{{}}}", self.name, fields_str)
    }
}

pub struct Evaluation {
    forward_name: String,
    backward_name: String,
    fields: Vec<(String, Field)>,
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
    name: String,
    fields: Vec<(String, Field)>,
}

impl Serialize for Emotion {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!("emotion {} {{{}}}", self.name, fields_str)
    }
}

pub struct Practice {
    name: String,
    params: Vec<String>,
    display: Option<String>,
    actions: Option<Vec<PracticeAction>>,
    fields: Vec<(String, Field)>,
}

pub struct PracticeAction {
    for_actor: String,
    name: String,
    conditions: Vec<PracticeCondition>,
    outcomes: Vec<PracticeOutcome>,
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

fn parse_praxsmth(input_str: &str) -> Result<Vec<PraxsmthValue>, Error<Rule>> {
    let pairs = PraxsmthParser::parse(Rule::praxsmth, input_str)?;

    fn parse_agent_inner(pair: Pair<Rule>) -> Agent {
        // pair is a Rule::agent_inner, which is: var_name ~ agent_brackets?
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();

        // Check if there's an agent_brackets
        let subagents = if let Some(brackets) = inner.next() {
            brackets.into_inner().map(parse_agent_inner).collect()
        } else {
            Vec::new()
        };

        Agent { name, subagents }
    }

    fn parse_field(pair: Pair<Rule>) -> Field {
        // pair is either Rule::number_range or Rule::variant_list
        match pair.as_rule() {
            Rule::number_range => {
                // number_range is: number ~ ".." ~ number
                let mut numbers = pair.into_inner();
                let start: i64 = numbers.next().unwrap().as_str().parse().unwrap();
                let end: i64 = numbers.next().unwrap().as_str().parse().unwrap();
                Field::NumberRange(start, end)
            }
            Rule::variant_list => {
                // variant_list is: var_name ~ ("|" ~ var_name)+
                let variants = pair
                    .into_inner()
                    .map(|var| var.as_str().to_string())
                    .collect();
                Field::VariantList(variants)
            }
            _ => unreachable!(),
        }
    }

    fn parse_field_brackets(pair: Pair<Rule>) -> Vec<(String, Field)> {
        // pair is Rule::field_brackets, contains field_def pairs
        pair.into_inner()
            .map(|field_def| {
                // kv_pair is: var_name ~ ":" ~ value
                let mut field_def_inner = field_def.into_inner();
                let field_name = field_def_inner.next().unwrap().as_str().to_string();
                let field = parse_field(field_def_inner.next().unwrap());
                (field_name, field)
            })
            .collect()
    }

    fn parse_trait(pair: Pair<Rule>) -> Trait {
        // pair is Rule::trait_def: "trait" ~ var_name ~ trait_brackets?
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();

        // Check if there's a field_brackets
        let fields = if let Some(brackets) = inner.next() {
            // brackets is Rule::field_brackets, contains field_def pairs
            parse_field_brackets(brackets)
        } else {
            Vec::new()
        };

        Trait { name, fields }
    }

    fn parse_directional(pair: Pair<Rule>) -> Directional {
        // pair is Rule::directional_def: "directional" ~ var_name ~ var_name ~ directional_brackets?
        let mut inner = pair.into_inner();
        let forward_name = inner.next().unwrap().as_str().to_string();
        let backward_name = inner.next().unwrap().as_str().to_string();

        // Check if there's a field_brackets
        let fields = if let Some(brackets) = inner.next() {
            // brackets is Rule::field_brackets, contains field_def pairs
            parse_field_brackets(brackets)
        } else {
            Vec::new()
        };

        Directional {
            forward_name,
            backward_name,
            fields,
        }
    }

    fn parse_reciprocal(pair: Pair<Rule>) -> Reciprocal {
        // pair is Rule::reciprocal_def: "reciprocal" ~ var_name ~ reciprocal_brackets?
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();

        // Check if there's a field_brackets
        let fields = if let Some(brackets) = inner.next() {
            // brackets is Rule::field_brackets, contains field_def pairs
            parse_field_brackets(brackets)
        } else {
            Vec::new()
        };

        Reciprocal { name, fields }
    }

    fn parse_evaluation(pair: Pair<Rule>) -> Evaluation {
        // pair is Rule::evaluation_def: "evaluation" ~ var_name ~ var_name ~ evaluation_brackets?
        let mut inner = pair.into_inner();
        let forward_name = inner.next().unwrap().as_str().to_string();
        let backward_name = inner.next().unwrap().as_str().to_string();

        // Check if there's a field_brackets
        let fields = if let Some(brackets) = inner.next() {
            // brackets is Rule::field_brackets, contains field_def pairs
            parse_field_brackets(brackets)
        } else {
            Vec::new()
        };

        Evaluation {
            forward_name,
            backward_name,
            fields,
        }
    }

    fn parse_emotion(pair: Pair<Rule>) -> Emotion {
        // pair is Rule::emotion_def: "emotion" ~ var_name ~ emotion_brackets?
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();

        // Check if there's a field_brackets
        let fields = if let Some(brackets) = inner.next() {
            // brackets is Rule::field_brackets, contains field_def pairs
            parse_field_brackets(brackets)
        } else {
            Vec::new()
        };

        Emotion { name, fields }
    }

    fn parse_practice_condition(pairs: Pair<Rule>, pratt: &PrattParser<Rule>) -> PracticeCondition {
        pratt
            .map_primary(|primary| match primary.as_rule() {
                Rule::sentence => PracticeCondition::Sentence(primary.as_str().to_string()),
                _ => unreachable!(),
            })
            .map_prefix(|op, rhs| match op.as_rule() {
                Rule::not => PracticeCondition::Not(Box::new(rhs)),
                _ => unreachable!(),
            })
            .map_infix(|lhs, op, rhs| match op.as_rule() {
                Rule::and => PracticeCondition::And(Box::new(lhs), Box::new(rhs)),
                Rule::or => PracticeCondition::Or(Box::new(lhs), Box::new(rhs)),
                Rule::is => PracticeCondition::Is(Box::new(lhs), Box::new(rhs)),
                _ => unreachable!(),
            })
            .parse(pairs.into_inner())
    }

    fn parse_practice_outcome(pair: Pair<Rule>) -> PracticeOutcome {
        // pair is one of the outcome_* rules
        let mut inner = pair.clone().into_inner();

        match pair.as_rule() {
            Rule::outcome_print => {
                // "print" ~ string
                let string_pair = inner.next().unwrap();
                PracticeOutcome::Print(string_pair.as_str().trim_matches('"').to_string())
            }
            Rule::outcome_delete => {
                // "delete" ~ sentence
                let sentence_pair = inner.next().unwrap();
                PracticeOutcome::Delete(sentence_pair.as_str().to_string())
            }
            Rule::outcome_set => {
                // "set" ~ sentence ~ "to" ~ value
                let sentence_pair = inner.next().unwrap();
                let value_pair = inner.next().unwrap();
                let value_str = match value_pair.as_rule() {
                    Rule::string => value_pair.as_str().trim_matches('"').to_string(),
                    Rule::number => value_pair.as_str().to_string(),
                    _ => unreachable!(),
                };
                PracticeOutcome::Set(sentence_pair.as_str().to_string(), value_str)
            }
            Rule::outcome_increase => {
                // "increase" ~ sentence ~ "by" ~ number
                let sentence_pair = inner.next().unwrap();
                let number_pair = inner.next().unwrap();
                let num: i64 = number_pair.as_str().parse().unwrap();
                PracticeOutcome::Increase(sentence_pair.as_str().to_string(), num)
            }
            Rule::outcome_cycle => {
                // "cycle" ~ sentence ~ "by" ~ number
                let sentence_pair = inner.next().unwrap();
                let number_pair = inner.next().unwrap();
                let num: i64 = number_pair.as_str().parse().unwrap();
                PracticeOutcome::Cycle(sentence_pair.as_str().to_string(), num)
            }
            _ => unreachable!(),
        }
    }

    fn parse_practice_action(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> PracticeAction {
        // pair is Rule::practice_action: "{" ~ practice_action_field_def ~ ... ~ "}"
        let mut for_actor = String::new();
        let mut name = String::new();
        let mut conditions = Vec::new();
        let mut outcomes = Vec::new();

        for field_pair in pair.into_inner() {
            // field_pair is one of the practice_* field rules
            let mut inner = field_pair.clone().into_inner();

            match field_pair.as_rule() {
                Rule::practice_for => {
                    // "for" ~ ":" ~ var_name
                    let var_pair = inner.next().unwrap();
                    for_actor = var_pair.as_str().to_string();
                }
                Rule::practice_name => {
                    // "name" ~ ":" ~ string
                    let string_pair = inner.next().unwrap();
                    name = string_pair.as_str().trim_matches('"').to_string();
                }
                Rule::practice_conditions_field => {
                    // "conditions" ~ ":" ~ practice_conditions
                    let conditions_pair = inner.next().unwrap(); // Rule::practice_conditions
                    conditions = conditions_pair
                        .into_inner()
                        .map(|cond| parse_practice_condition(cond, pratt))
                        .collect();
                }
                Rule::practice_outcomes_field => {
                    // "outcomes" ~ ":" ~ practice_outcomes
                    let outcomes_pair = inner.next().unwrap(); // Rule::practice_outcomes
                    outcomes = outcomes_pair
                        .into_inner()
                        .map(parse_practice_outcome)
                        .collect();
                }
                _ => unreachable!(),
            }
        }

        PracticeAction {
            for_actor,
            name,
            conditions,
            outcomes,
        }
    }

    let practice_pratt = PrattParser::new()
        .op(Op::infix(Rule::and, Assoc::Left) | Op::infix(Rule::or, Assoc::Left))
        .op(Op::infix(Rule::is, Assoc::Left))
        .op(Op::prefix(Rule::not));

    fn parse_practice(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> Practice {
        // pair is Rule::practice_def: "practice" ~ var_name ~ practice_params ~ practice_brackets
        let mut inner = pair.into_inner();

        // Get practice name
        let name = inner.next().unwrap().as_str().to_string();

        // Get practice params
        let params_pair = inner.next().unwrap(); // Rule::practice_params
        let params: Vec<String> = params_pair
            .into_inner()
            .map(|p| p.as_str().to_string())
            .collect();

        // Get practice brackets (fields)
        let brackets_pair = inner.next().unwrap(); // Rule::practice_brackets

        let mut display = None;
        let mut actions = None;
        let mut fields = Vec::new();

        for field_pair in brackets_pair.into_inner() {
            // field_pair is one of the practice_* field rules
            let mut field_inner = field_pair.clone().into_inner();

            match field_pair.as_rule() {
                Rule::practice_display => {
                    // "display" ~ ":" ~ string
                    let string_pair = field_inner.next().unwrap();
                    display = Some(string_pair.as_str().trim_matches('"').to_string());
                }
                Rule::practice_actions_field => {
                    // "actions" ~ ":" ~ practice_actions
                    let actions_pair = field_inner.next().unwrap(); // Rule::practice_actions
                    actions = Some(
                        actions_pair
                            .into_inner()
                            .map(|action| parse_practice_action(action, pratt))
                            .collect(),
                    );
                }
                Rule::practice_generic_field => {
                    // var_name ~ ":" ~ field
                    let field_name = field_inner.next().unwrap().as_str().to_string();
                    let field_value = parse_field(field_inner.next().unwrap());
                    fields.push((field_name, field_value));
                }
                _ => unreachable!(),
            }
        }

        Practice {
            name,
            params,
            display,
            actions,
            fields,
        }
    }

    let values = pairs
        .filter(|pair| {
            matches!(
                pair.as_rule(),
                Rule::agent_def
                    | Rule::trait_def
                    | Rule::directional_def
                    | Rule::reciprocal_def
                    | Rule::evaluation_def
                    | Rule::emotion_def
                    | Rule::practice_def
            )
        })
        .map(|pair| match pair.as_rule() {
            Rule::agent_def => {
                // agent_def is: "agent" ~ agent_inner
                let agent_inner = pair.into_inner().next().unwrap();
                PraxsmthValue::Agent(parse_agent_inner(agent_inner))
            }
            Rule::trait_def => PraxsmthValue::Trait(parse_trait(pair)),
            Rule::directional_def => PraxsmthValue::Directional(parse_directional(pair)),
            Rule::reciprocal_def => PraxsmthValue::Reciprocal(parse_reciprocal(pair)),
            Rule::evaluation_def => PraxsmthValue::Evaluation(parse_evaluation(pair)),
            Rule::emotion_def => PraxsmthValue::Emotion(parse_emotion(pair)),
            Rule::practice_def => PraxsmthValue::Practice(parse_practice(pair, &practice_pratt)),
            _ => unreachable!(),
        })
        .collect();

    Ok(values)
}

pub fn test_parse() {
    let unparsed_file = fs::read_to_string("examples.txt").expect("cannot read file");

    let values: Vec<PraxsmthValue> = parse_praxsmth(&unparsed_file).expect("unsuccessful parse");

    println!(
        "Output:\n\n{}",
        values
            .iter()
            .map(|v| v.serialize())
            .collect::<Vec<_>>()
            .join("\n")
    );
}
