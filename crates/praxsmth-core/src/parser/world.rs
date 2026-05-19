use std::collections::HashMap;

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest::pratt_parser::PrattParser;

use crate::definitions::world::*;
use crate::parser::{
    PraxsmthParser, Rule, build_expression_pratt, parse_constant, parse_expression, parse_sentence,
};

fn parse_agent_goal(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> Goal {
    // pair is Rule::w_agent_goal:
    //   "goal" ~ "(" ~ number ~ ")" ~ ":" ~ (w_increase | w_decrease)? ~ expression
    let mut inner = pair.into_inner();
    let weight: f64 = inner.next().unwrap().as_str().parse().unwrap();

    let next = inner.next().unwrap();
    let (measurement, expression_pair) = match next.as_rule() {
        Rule::w_increase => (GoalMeasurement::Increase, inner.next().unwrap()),
        Rule::w_decrease => (GoalMeasurement::Decrease, inner.next().unwrap()),
        _ => (GoalMeasurement::Exists, next),
    };
    let expression = parse_expression(expression_pair, pratt);

    Goal {
        weight,
        measurement,
        expression,
    }
}

fn parse_agent(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> AgentInfo {
    // pair is Rule::w_agent:
    //   "agent" ~ var_name ~ ("as" ~ string)? ~ w_agent_inactive? ~ w_agent_brackets?
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
            Rule::w_agent_inactive => {
                active = false;
            }
            Rule::w_agent_brackets => {
                for goal_pair in next.into_inner() {
                    goals.push(parse_agent_goal(goal_pair, pratt));
                }
            }
            _ => unreachable!(),
        }
    }

    AgentInfo {
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

pub fn parse_world(input_str: &str) -> Result<Vec<PraxsmthWorldDefinition>, Error<Rule>> {
    let pairs = PraxsmthParser::parse(Rule::praxsmth_world, input_str)?;
    let pratt = build_expression_pratt();

    let values = pairs
        .filter(|pair| matches!(pair.as_rule(), Rule::w_agent | Rule::w_declaration))
        .map(|pair| match pair.as_rule() {
            Rule::w_agent => PraxsmthWorldDefinition::AgentInfo(parse_agent(pair, &pratt)),
            Rule::w_declaration => PraxsmthWorldDefinition::Declaration(parse_declaration(pair)),
            _ => unreachable!(),
        })
        .collect();

    Ok(values)
}
