use std::collections::HashMap;

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};

use crate::definitions::{FieldTypes, types::*};
use crate::parser::world::parse_declaration;
use crate::parser::{PraxsmthParser, Rule, parse_field, parse_sentence, parse_string, parse_value};

fn parse_field_brackets(pair: Pair<Rule>) -> FieldTypes {
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

fn parse_trait(pair: Pair<Rule>) -> PraxsmthType {
    // pair is Rule::t_trait: "trait" ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        HashMap::new()
    };

    PraxsmthType {
        name,
        fields,
        data: PraxsmthTypeData::Trait,
    }
}

fn parse_directional(pair: Pair<Rule>) -> PraxsmthType {
    // pair is Rule::t_directional: "directional" ~ var_name ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let forward_name = inner.next().unwrap().as_str().to_string();
    let backward_name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        HashMap::new()
    };

    PraxsmthType {
        name: forward_name.clone(),
        fields: fields.clone(),
        data: PraxsmthTypeData::Directional {
            complement: backward_name.clone(),
        },
    }
}

fn parse_reciprocal(pair: Pair<Rule>) -> PraxsmthType {
    // pair is Rule::t_reciprocal: "reciprocal" ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        HashMap::new()
    };

    PraxsmthType {
        name,
        fields,
        data: PraxsmthTypeData::Reciprocal,
    }
}

fn parse_evaluation(pair: Pair<Rule>) -> PraxsmthType {
    // pair is Rule::t_evaluation: "evaluation" ~ var_name ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let forward_name = inner.next().unwrap().as_str().to_string();
    let backward_name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        HashMap::new()
    };

    PraxsmthType {
        name: forward_name.clone(),
        fields: fields.clone(),
        data: PraxsmthTypeData::Evaluation {
            complement: backward_name.clone(),
        },
    }
}

fn parse_emotion(pair: Pair<Rule>) -> PraxsmthType {
    // pair is Rule::t_emotion: "emotion" ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        HashMap::new()
    };

    PraxsmthType {
        name,
        fields,
        data: PraxsmthTypeData::Emotion,
    }
}

fn parse_practice_condition(pairs: Pair<Rule>, pratt: &PrattParser<Rule>) -> PracticeCondition {
    pratt
        .map_primary(|primary| PracticeCondition::Value(parse_value(primary)))
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
        Rule::outcome_broadcast => {
            // "broadcast" ~ string
            let string_pair = inner.next().unwrap();
            PracticeOutcome::Broadcast(parse_string(string_pair))
        }
        Rule::outcome_say => {
            // "say" ~ string
            let string_pair = inner.next().unwrap();
            PracticeOutcome::Say(parse_string(string_pair))
        }
        Rule::outcome_delete => {
            // "delete" ~ sentence
            let sentence_pair = inner.next().unwrap();
            PracticeOutcome::Delete(parse_sentence(sentence_pair))
        }
        Rule::outcome_set => {
            // "set" ~ w_declaration
            let decl_pair = inner.next().unwrap();
            PracticeOutcome::Set(parse_declaration(decl_pair))
        }
        Rule::outcome_update => {
            // "update" ~ sentence ~ "to" ~ value
            let sentence_pair = inner.next().unwrap();
            let value_pair = inner.next().unwrap();
            PracticeOutcome::Update(parse_sentence(sentence_pair), parse_value(value_pair))
        }
        Rule::outcome_increase => {
            // "increase" ~ sentence ~ "by" ~ number
            let sentence_pair = inner.next().unwrap();
            let number_pair = inner.next().unwrap();
            let num: i64 = number_pair.as_str().parse().unwrap();
            PracticeOutcome::Increase(parse_sentence(sentence_pair), num)
        }
        Rule::outcome_cycle => {
            // "cycle" ~ sentence ~ "by" ~ number
            let sentence_pair = inner.next().unwrap();
            let number_pair = inner.next().unwrap();
            let num: i64 = number_pair.as_str().parse().unwrap();
            PracticeOutcome::Cycle(parse_sentence(sentence_pair), num)
        }
        _ => unreachable!(),
    }
}

fn parse_practice_action(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> PracticeAction {
    // pair is Rule::t_practice_action: "{" ~ t_practice_action_field_def ~ ... ~ "}"
    let mut for_actor = String::new();
    let mut name = String::new();
    let mut conditions = Vec::new();
    let mut outcomes = Vec::new();

    for field_pair in pair.into_inner() {
        // field_pair is one of the t_practice_* field rules
        let mut inner = field_pair.clone().into_inner();

        match field_pair.as_rule() {
            Rule::t_practice_for => {
                // "for" ~ ":" ~ var_name
                let var_pair = inner.next().unwrap();
                for_actor = var_pair.as_str().to_string();
            }
            Rule::t_practice_name => {
                // "name" ~ ":" ~ string
                let string_pair = inner.next().unwrap();
                name = parse_string(string_pair);
            }
            Rule::t_practice_conditions_field => {
                // "conditions" ~ ":" ~ t_practice_conditions
                let conditions_pair = inner.next().unwrap(); // Rule::practice_conditions
                conditions = conditions_pair
                    .into_inner()
                    .map(|cond| parse_practice_condition(cond, pratt))
                    .collect();
            }
            Rule::t_practice_outcomes_field => {
                // "outcomes" ~ ":" ~ t_practice_outcomes
                let outcomes_pair = inner.next().unwrap(); // Rule::t_practice_outcomes
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

fn parse_practice(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> PraxsmthType {
    // pair is Rule::t_practice: "practice" ~ var_name ~ t_practice_params ~ t_practice_brackets
    let mut inner = pair.into_inner();

    // Get practice name
    let name = inner.next().unwrap().as_str().to_string();

    // Get practice params
    let params_pair = inner.next().unwrap(); // Rule::t_practice_params
    let params: Vec<String> = params_pair
        .into_inner()
        .map(|p| p.as_str().to_string())
        .collect();

    // Get practice brackets (fields)
    let brackets_pair = inner.next().unwrap(); // Rule::t_practice_brackets

    let mut display = None;
    let mut actions = Vec::new();
    let mut fields = HashMap::new();

    for field_pair in brackets_pair.into_inner() {
        // field_pair is one of the t_practice_* field rules
        let mut field_inner = field_pair.clone().into_inner();

        match field_pair.as_rule() {
            Rule::t_practice_display => {
                // "display" ~ ":" ~ string
                let string_pair = field_inner.next().unwrap();
                display = Some(parse_string(string_pair));
            }
            Rule::t_practice_actions_field => {
                // "actions" ~ ":" ~ t_practice_actions
                let actions_pair = field_inner.next().unwrap(); // Rule::t_practice_actions
                actions = actions_pair
                    .into_inner()
                    .map(|action| parse_practice_action(action, pratt))
                    .collect();
            }
            Rule::t_practice_generic_field => {
                // var_name ~ ":" ~ field
                let field_name = field_inner.next().unwrap().as_str().to_string();
                let field_value = parse_field(field_inner.next().unwrap());
                fields.insert(field_name, field_value);
            }
            _ => unreachable!(),
        }
    }

    PraxsmthType {
        name,
        fields,
        data: PraxsmthTypeData::Practice {
            params,
            display,
            actions,
        },
    }
}

pub fn parse_types(input_str: &str) -> Result<Vec<PraxsmthType>, Error<Rule>> {
    let pairs = PraxsmthParser::parse(Rule::praxsmth_types, input_str)?;

    let practice_pratt = PrattParser::new()
        .op(Op::infix(Rule::and, Assoc::Left) | Op::infix(Rule::or, Assoc::Left))
        .op(Op::infix(Rule::is, Assoc::Left))
        .op(Op::prefix(Rule::not));

    let values = pairs
        .filter(|pair| {
            matches!(
                pair.as_rule(),
                Rule::t_trait
                    | Rule::t_directional
                    | Rule::t_reciprocal
                    | Rule::t_evaluation
                    | Rule::t_emotion
                    | Rule::t_practice
            )
        })
        .map(|pair| match pair.as_rule() {
            Rule::t_trait => parse_trait(pair),
            Rule::t_directional => parse_directional(pair),
            Rule::t_reciprocal => parse_reciprocal(pair),
            Rule::t_evaluation => parse_evaluation(pair),
            Rule::t_emotion => parse_emotion(pair),
            Rule::t_practice => parse_practice(pair, &practice_pratt),
            _ => unreachable!(),
        })
        .collect();

    Ok(values)
}
