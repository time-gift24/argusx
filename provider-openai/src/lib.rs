pub const VERSION: &str = "0.1.0";

pub mod chunk;
pub mod parser;

pub use parser::parse_chunk;
