use crate::{Error, ProviderConfig};

pub struct ProviderClient {
    _http: reqwest::Client,
    _config: ProviderConfig,
}

impl ProviderClient {
    pub fn new(config: ProviderConfig) -> Result<Self, Error> {
        if config.base_url.trim().is_empty() {
            return Err(Error::Config("base_url is required".into()));
        }
        if config.api_key.trim().is_empty() {
            return Err(Error::Config("api_key is required".into()));
        }

        let _ = config.dialect;

        Ok(Self {
            _http: reqwest::Client::new(),
            _config: config,
        })
    }
}
