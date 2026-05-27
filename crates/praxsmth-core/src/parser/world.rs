use std::{collections::HashMap, fmt};

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest::pratt_parser::PrattParser;

use crate::{
    parser::{
        PraxsmthParser, Rule, build_expression_pratt, parse_constant, parse_expression,
        parse_sentence,
    },
    world::{
        ActorInitInfo,
        goals::{Goal, GoalMeasurement},
        simulation::Declaration,
    },
};

#[derive(Debug, Clone)]
pub enum WorldInitStep {
    NewActor(ActorInitInfo),
    NewRelation(Declaration),
}

impl fmt::Display for WorldInitStep {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WorldInitStep::NewActor(a) => write!(f, "{}", a),
            WorldInitStep::NewRelation(d) => write!(f, "{}", d),
        }
    }
}

fn parse_actor_goal(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> Goal {
    // pair is Rule::w_actor_goal:
    //   "goal" ~ "(" ~ number ~ ")" ~ ":" ~ (w_increase | w_decrease)? ~ expression
    let mut inner = pair.into_inner();
    let weight: f64 = inner.next().unwrap().as_str().parse().unwrap();

    let next = inner.next().unwrap();
    let (measurement, expression_pair) = match next.as_rule() {
        Rule::w_delta => (GoalMeasurement::Delta, inner.next().unwrap()),
        _ => (GoalMeasurement::Exists, next),
    };
    let expression = parse_expression(expression_pair, pratt);

    Goal {
        weight,
        measurement,
        expression,
    }
}

fn parse_actor(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> ActorInitInfo {
    // pair is Rule::w_actor:
    //   "actor" ~ var_name ~ ("as" ~ string)? ~ w_actor_inactive? ~ w_actor_brackets?
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let mut name = id.clone();
    let mut active = true;
    let mut goals = Vec::new();

    for next in inner {
        match next.as_rule() {
            Rule::string => {
                name = next.into_inner().next().unwrap().as_str().to_string();
            }
            Rule::w_actor_inactive => {
                active = false;
            }
            Rule::w_actor_brackets => {
                for goal_pair in next.into_inner() {
                    goals.push(parse_actor_goal(goal_pair, pratt));
                }
            }
            _ => unreachable!(),
        }
    }

    ActorInitInfo {
        id,
        name,
        active,
        goals,
    }
}

pub fn parse_declaration(pair: Pair<Rule>) -> Declaration {
    // pair is Rule::w_declaration: sentence ~ w_decl_brackets?
    let mut inner = pair.into_inner();

    let sentence_pair = inner.next().unwrap();
    let sentence = parse_sentence(sentence_pair);

    let mut fields = HashMap::new();

    if let Some(brackets) = inner.next() {
        // brackets is Rule::w_decl_brackets: "{" ~ w_decl_field_def* ~ "}"
        for field_def in brackets.into_inner() {
            // field_def is Rule::w_decl_field_def: var_name ~ ":" ~ const
            let mut field_inner = field_def.into_inner();
            let field_name = field_inner.next().unwrap().as_str().to_string();
            let field_value = parse_constant(field_inner.next().unwrap());
            fields.insert(field_name, field_value);
        }
    }

    Declaration { sentence, fields }
}

pub fn parse_world(input_str: &str) -> Result<Vec<WorldInitStep>, Error<Rule>> {
    let pairs = PraxsmthParser::parse(Rule::praxsmth_world, input_str)?;
    let pratt = build_expression_pratt();

    let values = pairs
        .filter(|pair| matches!(pair.as_rule(), Rule::w_actor | Rule::w_declaration))
        .map(|pair| match pair.as_rule() {
            Rule::w_actor => WorldInitStep::NewActor(parse_actor(pair, &pratt)),
            Rule::w_declaration => WorldInitStep::NewRelation(parse_declaration(pair)),
            _ => unreachable!(),
        })
        .collect();

    Ok(values)
}
