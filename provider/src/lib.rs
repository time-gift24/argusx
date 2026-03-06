mod client;
pub const VERSION: &str = "0.1.0";

pub mod dialect;
pub mod error;
pub mod normalize;
pub mod transport;
mod request;

pub use client::ProviderClient;
pub use error::{Error, ErrorKind, StreamError};
pub use request::Request;

use argus_core::ResponseEvent;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum Dialect {
    Openai,
    Zai,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub dialect: Dialect,
    pub base_url: String,
    pub api_key: String,
    pub headers: HashMap<String, String>,
}

enum InnerMapper {
    Openai(dialect::openai::mapper::Mapper),
    Zai(dialect::zai::mapper::Mapper),
}

pub struct Mapper {
    inner: InnerMapper,
}

impl Mapper {
    pub fn new(dialect: Dialect) -> Self {
        let inner = match dialect {
            Dialect::Openai => InnerMapper::Openai(dialect::openai::mapper::Mapper::new()),
            Dialect::Zai => InnerMapper::Zai(dialect::zai::mapper::Mapper::new()),
        };
        Self { inner }
    }

    pub fn feed(&mut self, raw: &str) -> Result<Vec<ResponseEvent>, Error> {
        match &mut self.inner {
            InnerMapper::Openai(mapper) => Ok(mapper.feed(raw)?),
            InnerMapper::Zai(mapper) => Ok(mapper.feed(raw)?),
        }
    }

    pub fn on_done(&mut self) -> Result<Vec<ResponseEvent>, Error> {
        match &mut self.inner {
            InnerMapper::Openai(mapper) => Ok(mapper.on_done()?),
            InnerMapper::Zai(mapper) => Ok(mapper.on_done()?),
        }
    }
}
