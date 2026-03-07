use chromiumoxide::{Browser, BrowserConfig};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub enum ChromeState {
    NotInitialized,
    WaitingForUserConfirm,
    Starting,
    Ready(Arc<Mutex<Browser>>),
    Error(String),
}

pub struct ChromeManager {
    state: ChromeState,
    config: super::config::BrowserConfig,
    config_manager: super::config::BrowserConfigManager,
}

impl ChromeManager {
    pub fn new(
        config: super::config::BrowserConfig,
        config_manager: super::config::BrowserConfigManager,
    ) -> Self {
        Self {
            state: ChromeState::NotInitialized,
            config,
            config_manager,
        }
    }

    pub fn is_port_open(port: u16) -> bool {
        TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
    }

    pub fn needs_user_confirmation(&self) -> bool {
        matches!(self.state, ChromeState::WaitingForUserConfirm)
    }

    pub fn set_waiting_for_confirmation(&mut self) {
        self.state = ChromeState::WaitingForUserConfirm;
    }

    pub fn set_starting(&mut self) {
        self.state = ChromeState::Starting;
    }

    pub async fn connect_or_launch(&mut self) -> Result<Arc<Mutex<Browser>>, String> {
        let port = self.config.port;

        // Try to connect to existing Chrome
        if Self::is_port_open(port) {
            match Browser::connect(format!("http://localhost:{}", port)).await {
                Ok((browser, _)) => {
                    self.state = ChromeState::Ready(Arc::new(Mutex::new(browser)));
                    if let Ok(mut cfg) = self.config_manager.get_config() {
                        cfg.is_enabled = true;
                        let _ = self.config_manager.update_config(&cfg);
                    }
                    return self.get_browser();
                }
                Err(e) => {
                    tracing::warn!("Port open but connection failed: {}", e);
                }
            }
        }

        // Launch new Chrome
        self.launch_chrome().await
    }

    pub async fn launch_chrome(&mut self) -> Result<Arc<Mutex<Browser>>, String> {
        let mut config_builder = BrowserConfig::builder()
            .port(self.config.port)
            .no_sandbox(); // May be needed in some environments

        // If headless is false, show the browser window
        if !self.config.headless {
            config_builder = config_builder.with_head();
        }

        let config_builder = if let Some(ref path) = self.config.chrome_path {
            config_builder.chrome_executable(Path::new(path))
        } else {
            config_builder
        };

        let config_builder = if let Some(ref profile) = self.config.profile_dir {
            config_builder.user_data_dir(Path::new(profile))
        } else {
            config_builder
        };

        let config = match config_builder.build() {
            Ok(c) => c,
            Err(e) => return Err(format!("Failed to build config: {}", e)),
        };

        let (browser, _handler) = Browser::launch(config)
            .await
            .map_err(|e| format!("Failed to launch Chrome: {}", e))?;

        self.state = ChromeState::Ready(Arc::new(Mutex::new(browser)));

        // Update config as enabled
        if let Ok(mut cfg) = self.config_manager.get_config() {
            cfg.is_enabled = true;
            let _ = self.config_manager.update_config(&cfg);
        }

        self.get_browser()
    }

    fn get_browser(&self) -> Result<Arc<Mutex<Browser>>, String> {
        match &self.state {
            ChromeState::Ready(browser) => Ok(browser.clone()),
            _ => Err("Browser not ready".to_string()),
        }
    }
}
