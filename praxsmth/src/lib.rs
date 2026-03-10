pub mod parser;
pub mod types;
pub mod world;

pub trait Serialize {
    fn serialize(&self) -> String;
}
