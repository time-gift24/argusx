pub const VERSION: &str = "0.1.0";

pub mod dialect;
pub mod error;

pub use error::Error;

use argus_core::ResponseEvent;

#[derive(Debug, Clone, Copy)]
pub enum Dialect {
    Openai,
    Zai,
}

enum InnerMapper {
    Openai(dialect::openai::mapper::Mapper),
    Zai,
}

pub struct Mapper {
    inner: InnerMapper,
}

impl Mapper {
    pub fn new(dialect: Dialect) -> Self {
        let inner = match dialect {
            Dialect::Openai => {
                InnerMapper::Openai(dialect::openai::mapper::Mapper::new("openai".to_string()))
            }
            Dialect::Zai => InnerMapper::Zai,
        };
        Self { inner }
    }

    pub fn feed(&mut self, raw: &str) -> Result<Vec<ResponseEvent>, Error> {
        match &mut self.inner {
            InnerMapper::Openai(mapper) => Ok(mapper.feed(raw)?),
            InnerMapper::Zai => Err(Error::ZaiNotImplemented),
        }
    }

    pub fn on_done(&mut self) -> Result<Vec<ResponseEvent>, Error> {
        match &mut self.inner {
            InnerMapper::Openai(mapper) => Ok(mapper.on_done()?),
            InnerMapper::Zai => Err(Error::ZaiNotImplemented),
        }
    }
}
