pub const VERSION: &str = "0.1.0";

pub mod chunk;
pub mod mapper;
pub mod parser;

pub use mapper::Mapper;
pub use parser::parse_chunk;
