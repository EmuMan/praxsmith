use std::collections::HashMap;

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest::pratt_parser::PrattParser;

use crate::expressions::Expression;
use crate::parser::{
    PraxsmthParser, Rule, build_expression_pratt, parse_expression, parse_field_type,
    parse_sentence, parse_string,
};
use crate::types::{FieldType, FieldTypes, PracticeAction, RelationType, RelationTypeData};
use crate::world::simulation::Effect;

fn parse_field_brackets(pair: Pair<Rule>) -> FieldTypes {
    // pair is Rule::field_brackets, contains field_def pairs
    pair.into_inner()
        .map(|field_def| {
            // kv_pair is: var_name ~ ":" ~ value
            let mut field_def_inner = field_def.into_inner();
            let field_name = field_def_inner.next().unwrap().as_str().to_string();
            let field = parse_field_type(field_def_inner.next().unwrap());
            (field_name, field)
        })
        .collect::<Vec<(String, FieldType)>>()
        .into()
}

fn parse_trait(pair: Pair<Rule>) -> RelationType {
    // pair is Rule::t_trait: "trait" ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        FieldTypes::new()
    };

    RelationType {
        name,
        fields,
        data: RelationTypeData::Trait,
    }
}

fn parse_directional(pair: Pair<Rule>) -> RelationType {
    // pair is Rule::t_directional: t_exclusive? ~ "directional" ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();

    let first = inner.next().unwrap();
    let (exclusive, type_id) = if first.as_rule() == Rule::t_exclusive {
        (true, inner.next().unwrap().as_str().to_string())
    } else {
        (false, first.as_str().to_string())
    };

    let fields = if let Some(brackets) = inner.next() {
        parse_field_brackets(brackets)
    } else {
        FieldTypes::new()
    };

    RelationType {
        name: type_id,
        fields,
        data: RelationTypeData::Directional { exclusive },
    }
}

fn parse_reciprocal(pair: Pair<Rule>) -> RelationType {
    // pair is Rule::t_reciprocal: "reciprocal" ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        FieldTypes::new()
    };

    RelationType {
        name,
        fields,
        data: RelationTypeData::Reciprocal,
    }
}

fn parse_emotion(pair: Pair<Rule>) -> RelationType {
    // pair is Rule::t_emotion: "emotion" ~ var_name ~ field_brackets?
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Check if there's a field_brackets
    let fields = if let Some(brackets) = inner.next() {
        // brackets is Rule::field_brackets, contains field_def pairs
        parse_field_brackets(brackets)
    } else {
        FieldTypes::new()
    };

    RelationType {
        name,
        fields,
        data: RelationTypeData::Emotion,
    }
}

pub fn parse_effect_brackets(
    pair: Pair<Rule>,
    pratt: &PrattParser<Rule>,
) -> HashMap<String, Expression> {
    // pair is Rule::effect_create_brackets: "{" ~ effect_create_field_asgn* ~ "}"

    let mut fields = HashMap::new();

    for field_asgn in pair.into_inner() {
        // field_def is Rule::effect_create_field_asgn: var_name ~ ":" ~ expression
        let mut field_inner = field_asgn.into_inner();
        let field_name = field_inner.next().unwrap().as_str().to_string();
        let field_value = parse_expression(field_inner.next().unwrap(), pratt);
        fields.insert(field_name, field_value);
    }

    fields
}

pub fn parse_effect(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> Effect {
    // pair is one of the effect_* rules
    let mut inner = pair.clone().into_inner();

    match pair.as_rule() {
        Rule::effect_broadcast => {
            // "broadcast" ~ expression
            let expr_pair = inner.next().unwrap();
            Effect::Broadcast(parse_expression(expr_pair, pratt))
        }
        Rule::effect_say => {
            // "say" ~ expression
            let expr_pair = inner.next().unwrap();
            Effect::Say(parse_expression(expr_pair, pratt))
        }
        Rule::effect_activate => {
            // "activate" ~ expression
            let expr_pair = inner.next().unwrap();
            Effect::Activate(parse_expression(expr_pair, pratt))
        }
        Rule::effect_deactivate => {
            // "deactivate" ~ expression
            let expr_pair = inner.next().unwrap();
            Effect::Deactivate(parse_expression(expr_pair, pratt))
        }
        Rule::effect_delete => {
            // "delete" ~ sentence
            let sentence_pair = inner.next().unwrap();
            Effect::Delete(parse_sentence(sentence_pair))
        }
        Rule::effect_create => {
            // "create" ~ sentence ~ effect_create_brackets?
            let sentence_pair = inner.next().unwrap();
            let brackets = match inner.next() {
                Some(brackets_pair) => {
                    // brackets_pair is Rule::effect_create_brackets
                    parse_effect_brackets(brackets_pair, pratt)
                }
                None => HashMap::new(),
            };
            Effect::Create(parse_sentence(sentence_pair), brackets)
        }
        Rule::effect_set => {
            // "set" ~ sentence ~ "to" ~ expression
            let sentence_pair = inner.next().unwrap();
            let value_pair = inner.next().unwrap();
            Effect::Set(
                parse_sentence(sentence_pair),
                parse_expression(value_pair, pratt),
            )
        }
        Rule::effect_increase => {
            // "increase" ~ sentence ~ "by" ~ number
            let sentence_pair = inner.next().unwrap();
            let number_pair = inner.next().unwrap();
            let num: f64 = number_pair.as_str().parse().unwrap();
            Effect::Increase(parse_sentence(sentence_pair), num)
        }
        Rule::effect_cycle => {
            // "cycle" ~ sentence ~ "by" ~ number
            let sentence_pair = inner.next().unwrap();
            let number_pair = inner.next().unwrap();
            let num: f64 = number_pair.as_str().parse().unwrap();
            Effect::Cycle(parse_sentence(sentence_pair), num)
        }
        _ => unreachable!(),
    }
}

fn parse_practice_action(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> PracticeAction {
    // pair is Rule::t_practice_action: "{" ~ t_practice_action_field_def ~ ... ~ "}"
    let mut for_actor = String::new();
    let mut name = String::new();
    let mut conditions = Vec::new();
    let mut effects = Vec::new();

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
                // "conditions" ~ ":" ~ t_condition_list
                let conditions_pair = inner.next().unwrap(); // Rule::t_condition_list
                let cond_inner = conditions_pair.into_inner();

                conditions = cond_inner
                    .map(|expr| parse_expression(expr, pratt))
                    .collect();
            }
            Rule::t_practice_outcomes_field => {
                // "outcomes" ~ ":" ~ t_practice_outcomes
                let outcomes_pair = inner.next().unwrap(); // Rule::t_practice_outcomes
                effects = outcomes_pair
                    .into_inner()
                    .map(|p| parse_effect(p, pratt))
                    .collect();
            }
            _ => unreachable!(),
        }
    }

    PracticeAction {
        for_actor,
        name,
        conditions,
        effects,
    }
}

fn parse_practice(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> RelationType {
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

    let mut actions = Vec::new();
    let mut fields = FieldTypes::new();

    for field_pair in brackets_pair.into_inner() {
        // field_pair is one of the t_practice_* field rules
        let mut field_inner = field_pair.clone().into_inner();

        match field_pair.as_rule() {
            Rule::t_practice_actions_field => {
                // "actions" ~ ":" ~ t_practice_actions
                let actions_pair = field_inner.next().unwrap(); // Rule::t_practice_actions
                actions = actions_pair
                    .into_inner()
                    .map(|action| parse_practice_action(action, pratt))
                    .collect();
            }
            Rule::t_practice_generic_field => {
                // var_name ~ ":" ~ field_type
                let field_name = field_inner.next().unwrap().as_str().to_string();
                let field_value = parse_field_type(field_inner.next().unwrap());
                fields.insert(field_name, field_value);
            }
            _ => unreachable!(),
        }
    }

    let mut self_id = vec!["practice".to_string()];
    self_id.push(name.to_string());
    self_id.extend(params.iter().cloned().map(String::from));

    log::info!("Parsed practice '{}', self_id: {:?}", name, self_id);

    RelationType {
        name,
        fields,
        data: RelationTypeData::Practice {
            self_id: self_id.into(),
            params,
            actions,
        },
    }
}

pub fn parse_types(input_str: &str) -> Result<Vec<RelationType>, Error<Rule>> {
    let pairs = PraxsmthParser::parse(Rule::praxsmth_types, input_str)?;

    let practice_pratt = build_expression_pratt();

    let values = pairs
        .filter(|pair| {
            matches!(
                pair.as_rule(),
                Rule::t_trait
                    | Rule::t_directional
                    | Rule::t_reciprocal
                    | Rule::t_emotion
                    | Rule::t_practice
            )
        })
        .map(|pair| match pair.as_rule() {
            Rule::t_trait => parse_trait(pair),
            Rule::t_directional => parse_directional(pair),
            Rule::t_reciprocal => parse_reciprocal(pair),
            Rule::t_emotion => parse_emotion(pair),
            Rule::t_practice => parse_practice(pair, &practice_pratt),
            _ => unreachable!(),
        })
        .collect();

    Ok(values)
}
