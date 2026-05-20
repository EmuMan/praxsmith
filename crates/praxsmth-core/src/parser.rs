use std::fs;

use pest::{
    iterators::Pair,
    pratt_parser::{Assoc, Op, PrattParser},
};
use pest_derive::Parser;

use crate::definitions::{
    PraxsmthConstant, PraxsmthField, PraxsmthValue, Sentence, Serialize, types::*,
};

pub mod types;
pub mod world;

#[derive(Parser)]
#[grammar = "praxsmth.pest"]
struct PraxsmthParser;

fn parse_string(pair: Pair<Rule>) -> String {
    // pair is Rule::string
    pair.as_str().trim_matches('"').to_string()
}

fn parse_sentence(pair: Pair<Rule>) -> Sentence {
    // pair is Rule::sentence
    pair.into_inner()
        .map(|token| token.as_str().to_string())
        .collect()
}

fn parse_value(pair: Pair<Rule>) -> PraxsmthValue {
    match pair.as_rule() {
        Rule::number => PraxsmthValue::Number(pair.as_str().parse().unwrap()),
        Rule::string => PraxsmthValue::String(parse_string(pair)),
        Rule::sentence => {
            let parts = parse_sentence(pair);
            if parts.len() == 1 {
                PraxsmthValue::Variant(parts.into_iter().next().unwrap())
            } else {
                PraxsmthValue::Variable(parts)
            }
        }
        _ => unreachable!(),
    }
}

pub fn parse_constant(pair: Pair<Rule>) -> PraxsmthConstant {
    match pair.as_rule() {
        Rule::number => PraxsmthConstant::Number(pair.as_str().parse().unwrap()),
        Rule::string => PraxsmthConstant::String(parse_string(pair)),
        Rule::var_name => PraxsmthConstant::Variant(pair.as_str().to_string()),
        _ => unreachable!(),
    }
}

fn parse_field(pair: Pair<Rule>) -> PraxsmthField {
    // pair is either Rule::number_range or Rule::variant_list
    match pair.as_rule() {
        Rule::number_range => {
            // number_range is: number ~ ".." ~ number
            let mut numbers = pair.into_inner();
            let start: f64 = numbers.next().unwrap().as_str().parse().unwrap();
            let end: f64 = numbers.next().unwrap().as_str().parse().unwrap();
            PraxsmthField::NumberRange(start, end)
        }
        Rule::variant_list => {
            // variant_list is: var_name ~ ("|" ~ var_name)+
            let variants = pair
                .into_inner()
                .map(|var| var.as_str().to_string())
                .collect();
            PraxsmthField::VariantList(variants)
        }
        _ => unreachable!(),
    }
}

pub fn test_parse() {
    let unparsed_types = fs::read_to_string("types.txt").expect("cannot read file");

    let values: Vec<PraxsmthType> =
        types::parse_types(&unparsed_types).expect("unsuccessful parse");

    println!(
        "Types Output:\n\n{}",
        values
            .iter()
            .map(|v| v.serialize())
            .collect::<Vec<_>>()
            .join("\n")
    );

    let unparsed_world = fs::read_to_string("world.txt").expect("cannot read file");

    let world_values = world::parse_world(&unparsed_world).expect("unsuccessful parse");

    println!(
        "\nWorld Output:\n\n{}",
        world_values
            .iter()
            .map(|v| v.serialize())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

pub fn build_expression_pratt() -> PrattParser<Rule> {
    PrattParser::new()
        .op(Op::infix(Rule::and, Assoc::Left) | Op::infix(Rule::or, Assoc::Left))
        .op(Op::infix(Rule::is, Assoc::Left))
        .op(Op::prefix(Rule::not))
}

pub fn parse_expression(pairs: Pair<Rule>, pratt: &PrattParser<Rule>) -> Expression {
    pratt
        .map_primary(|primary| Expression::Value(parse_value(primary)))
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::not => Expression::Not(Box::new(rhs)),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::and => Expression::And(Box::new(lhs), Box::new(rhs)),
            Rule::or => Expression::Or(Box::new(lhs), Box::new(rhs)),
            Rule::is => Expression::Is(Box::new(lhs), Box::new(rhs)),
            _ => unreachable!(),
        })
        .parse(pairs.into_inner())
}
