pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy)]
pub enum Dialect {
    Openai,
    Zai,
}

pub struct Mapper;

impl Mapper {
    pub fn new(_dialect: Dialect) -> Self {
        Self
    }
}
