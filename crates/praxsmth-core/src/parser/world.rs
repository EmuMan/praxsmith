use std::collections::HashMap;

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;

use crate::definitions::world::*;
use crate::parser::{PraxsmthParser, Rule, parse_constant, parse_sentence};

fn parse_agent_inner(pair: Pair<Rule>) -> AgentInfo {
    // pair is Rule::w_agent_inner: var_name ~ ("as" ~ string)? ~ w_agent_inactive? ~ w_agent_brackets?
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let mut name = id.clone();
    let mut active = true;
    let mut subagents = HashMap::new();

    for next in inner {
        match next.as_rule() {
            Rule::string => {
                // "as" ~ string
                name = next.into_inner().next().unwrap().as_str().to_string();
            }
            Rule::w_agent_inactive => {
                active = false;
            }
            Rule::w_agent_brackets => {
                // "{" ~ w_agent_inner* ~ "}"
                for agent_inner_pair in next.into_inner() {
                    let subagent = parse_agent_inner(agent_inner_pair);
                    subagents.insert(subagent.name.clone(), subagent);
                }
            }
            _ => unreachable!(),
        }
    }

    AgentInfo {
        id,
        name,
        active,
        subagents,
    }
}

fn parse_agent(pair: Pair<Rule>) -> AgentInfo {
    // pair is Rule::w_agent: "agent" ~ w_agent_inner
    let inner = pair.into_inner().next().unwrap();
    parse_agent_inner(inner)
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

    let values = pairs
        .filter(|pair| matches!(pair.as_rule(), Rule::w_agent | Rule::w_declaration))
        .map(|pair| match pair.as_rule() {
            Rule::w_agent => PraxsmthWorldDefinition::AgentInfo(parse_agent(pair)),
            Rule::w_declaration => PraxsmthWorldDefinition::Declaration(parse_declaration(pair)),
            _ => unreachable!(),
        })
        .collect();

    Ok(values)
}
