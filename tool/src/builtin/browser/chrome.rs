use chromiumoxide::{Browser, BrowserConfig, Handler};
use futures::StreamExt;
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct BrowserSession {
    browser: Arc<Mutex<Browser>>,
    handler_task: JoinHandle<()>,
}

pub enum ChromeState {
    NotInitialized,
    WaitingForUserConfirm,
    Starting,
    Ready(BrowserSession),
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
        self.replace_state(ChromeState::WaitingForUserConfirm);
    }

    pub fn set_starting(&mut self) {
        self.replace_state(ChromeState::Starting);
    }

    pub fn current_config(&self) -> Result<super::config::BrowserConfig, String> {
        self.config_manager
            .get_config()
            .map_err(|err| format!("Failed to load browser config: {err}"))
    }

    pub fn set_headless(&mut self, enabled: bool) -> Result<(), String> {
        self.config.headless = enabled;
        let mut config = self.current_config()?;
        config.headless = enabled;
        self.config_manager
            .update_config(&config)
            .map_err(|err| format!("Failed to update browser config: {err}"))
    }

    pub fn reset(&mut self) {
        self.replace_state(ChromeState::NotInitialized);
    }

    pub async fn connect_or_launch(&mut self) -> Result<Arc<Mutex<Browser>>, String> {
        if let ChromeState::Ready(session) = &self.state {
            return Ok(session.browser.clone());
        }

        let port = self.config.port;

        // Try to connect to existing Chrome
        if Self::is_port_open(port) {
            match Browser::connect(format!("http://localhost:{}", port)).await {
                Ok((browser, handler)) => return self.set_ready(browser, handler),
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

        let (browser, handler) = Browser::launch(config)
            .await
            .map_err(|e| format!("Failed to launch Chrome: {}", e))?;

        self.set_ready(browser, handler)
    }

    fn get_browser(&self) -> Result<Arc<Mutex<Browser>>, String> {
        match &self.state {
            ChromeState::Ready(session) => Ok(session.browser.clone()),
            _ => Err("Browser not ready".to_string()),
        }
    }

    fn set_ready(&mut self, browser: Browser, handler: Handler) -> Result<Arc<Mutex<Browser>>, String> {
        let browser = Arc::new(Mutex::new(browser));
        let mut handler = handler;
        let handler_task = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(err) = event {
                    tracing::warn!("browser handler stopped: {}", err);
                    break;
                }
            }
        });

        self.config.is_enabled = true;
        let mut config = self.current_config()?;
        config.is_enabled = true;
        self.config_manager
            .update_config(&config)
            .map_err(|err| format!("Failed to update browser config: {err}"))?;

        self.replace_state(ChromeState::Ready(BrowserSession {
            browser: browser.clone(),
            handler_task,
        }));

        self.get_browser()
    }

    fn replace_state(&mut self, next_state: ChromeState) {
        if let ChromeState::Ready(session) = std::mem::replace(&mut self.state, next_state) {
            session.handler_task.abort();
        }
    }
}
