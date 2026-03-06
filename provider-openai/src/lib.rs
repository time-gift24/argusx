pub const VERSION: &str = "0.1.0";

pub mod mapper;
pub mod parser;
pub mod schema;

pub use mapper::Mapper;
pub use parser::{parse_chunk, parse_payload, parse_sse_line};
