use std::fs;

use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "praxsmth.pest"]
struct PraxsmthParser;

trait Serialize {
    fn serialize(&self) -> String;
}

enum PraxsmthValue {
    Agent(Agent),
    Trait(Trait),
    Directional(Directional),
    Reciprocal(Reciprocal),
    Evaluation(Evaluation),
}

impl Serialize for PraxsmthValue {
    fn serialize(&self) -> String {
        match self {
            PraxsmthValue::Agent(a) => a.serialize(),
            PraxsmthValue::Trait(t) => t.serialize(),
            PraxsmthValue::Directional(d) => d.serialize(),
            PraxsmthValue::Reciprocal(r) => r.serialize(),
            PraxsmthValue::Evaluation(e) => e.serialize(),
        }
    }
}

struct Agent {
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

enum Field {
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

struct Trait {
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

struct Directional {
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

struct Reciprocal {
    name: String,
    fields: Vec<(String, Field)>,
}

impl Serialize for Reciprocal {
    fn serialize(&self) -> String {
        let fields_str = self.fields.serialize();
        format!("reciprocal {} {{{}}}", self.name, fields_str)
    }
}

struct Evaluation {
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

    let values = pairs
        .filter(|pair| {
            matches!(
                pair.as_rule(),
                Rule::agent_def
                    | Rule::trait_def
                    | Rule::directional_def
                    | Rule::reciprocal_def
                    | Rule::evaluation_def
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
