use std::fs;

use pest::iterators::Pair;
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
        Rule::var_name => PraxsmthValue::Variant(pair.as_str().to_string()),
        Rule::variable => PraxsmthValue::Variable(parse_sentence(pair)),
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
            let start: i64 = numbers.next().unwrap().as_str().parse().unwrap();
            let end: i64 = numbers.next().unwrap().as_str().parse().unwrap();
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

    let values: Vec<PraxsmthTypes> =
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
