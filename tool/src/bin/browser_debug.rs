use clap::{ArgAction, Parser, Subcommand};
use clap::error::ErrorKind;
use chromiumoxide::{Browser, BrowserConfig as ChromiumoxideBrowserConfig};
use futures::StreamExt;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;
use tool::builtin::browser::{
    config::BrowserConfig as ToolBrowserConfig,
    debug_port::{self, DEFAULT_DEBUG_PORT, DEFAULT_TIMEOUT_MS},
};

#[derive(Debug, PartialEq, Eq)]
enum DebugCommand {
    Connect {
        port: u16,
        url: Option<String>,
        print_cookies: bool,
        cookie_domain: Option<String>,
    },
    Launch {
        port: u16,
        headless: bool,
        chrome_path: Option<String>,
        url: Option<String>,
        print_cookies: bool,
        cookie_domain: Option<String>,
    },
    EnsureDebugPort {
        port: u16,
        timeout_ms: u64,
        chrome_path: Option<String>,
    },
}

#[derive(Debug, Parser)]
#[command(name = "browser_debug", about = "Temporary browser connection diagnostics")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Connect {
        #[arg(long, default_value_t = 9222)]
        port: u16,
        #[arg(long)]
        url: Option<String>,
        #[arg(long = "print-cookies", action = ArgAction::SetTrue)]
        print_cookies: bool,
        #[arg(long = "cookie-domain")]
        cookie_domain: Option<String>,
    },
    Launch {
        #[arg(long, default_value_t = 9222)]
        port: u16,
        #[arg(long, action = ArgAction::SetTrue)]
        headless: bool,
        #[arg(long)]
        chrome_path: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long = "print-cookies", action = ArgAction::SetTrue)]
        print_cookies: bool,
        #[arg(long = "cookie-domain")]
        cookie_domain: Option<String>,
    },
    EnsureDebugPort {
        #[arg(long, default_value_t = DEFAULT_DEBUG_PORT)]
        port: u16,
        #[arg(long, default_value_t = DEFAULT_TIMEOUT_MS)]
        timeout_ms: u64,
        #[arg(long)]
        chrome_path: Option<String>,
    },
}

fn parse_debug_command<I, T>(args: I) -> Result<DebugCommand, String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(err) if matches!(err.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) => {
            print!("{err}");
            std::process::exit(0);
        }
        Err(err) => return Err(err.to_string()),
    };
    Ok(match cli.command {
        Commands::Connect {
            port,
            url,
            print_cookies,
            cookie_domain,
        } => DebugCommand::Connect {
            port,
            url,
            print_cookies,
            cookie_domain,
        },
        Commands::Launch {
            port,
            headless,
            chrome_path,
            url,
            print_cookies,
            cookie_domain,
        } => DebugCommand::Launch {
            port,
            headless,
            chrome_path,
            url,
            print_cookies,
            cookie_domain,
        },
        Commands::EnsureDebugPort {
            port,
            timeout_ms,
            chrome_path,
        } => DebugCommand::EnsureDebugPort {
            port,
            timeout_ms,
            chrome_path,
        },
    })
}

#[tokio::main]
async fn main() {
    let command = match parse_debug_command(std::env::args_os()) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    let result = match command {
        DebugCommand::Connect {
            port,
            url,
            print_cookies,
            cookie_domain,
        } => run_connect(port, url, print_cookies, cookie_domain).await,
        DebugCommand::Launch {
            port,
            headless,
            chrome_path,
            url,
            print_cookies,
            cookie_domain,
        } => run_launch(port, headless, chrome_path, url, print_cookies, cookie_domain).await,
        DebugCommand::EnsureDebugPort {
            port,
            timeout_ms,
            chrome_path,
        } => run_ensure_debug_port(port, timeout_ms, chrome_path).await,
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run_connect(
    port: u16,
    url: Option<String>,
    print_cookies: bool,
    cookie_domain: Option<String>,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|err| format!("failed to build http client: {err}"))?;

    let endpoint = endpoint_base(port);
    println!("mode: connect");
    println!("endpoint: {endpoint}");

    let tcp_ok = probe_tcp(port).await;
    println!("tcp_connect: {}", if tcp_ok { "ok" } else { "failed" });
    if !tcp_ok {
        return Err(format!(
            "port {port} is not reachable. Chrome must be launched with --remote-debugging-port={port}"
        ));
    }

    let version = fetch_json(&client, &format!("{endpoint}/json/version")).await?;
    println!("json_version: {}", pretty_json(&version));

    let targets = fetch_json(&client, &format!("{endpoint}/json/list")).await?;
    println!("json_list: {}", pretty_json(&targets));

    let (browser, handler_task) = connect_browser(&endpoint).await?;
    print_pages(&browser).await?;

    if url.is_some() || print_cookies {
        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|err| format!("failed to create page on attached browser: {err}"))?;

        if let Some(url) = url.as_deref() {
            println!("navigate: {url}");
            page.goto(url)
                .await
                .map_err(|err| format!("failed to navigate to {url}: {err}"))?;
        }

        if print_cookies {
            print_cookies_for_page(&page, cookie_domain.as_deref()).await?;
        }
    }

    handler_task.abort();
    Ok(())
}

async fn run_launch(
    port: u16,
    headless: bool,
    chrome_path: Option<String>,
    url: Option<String>,
    print_cookies: bool,
    cookie_domain: Option<String>,
) -> Result<(), String> {
    println!("mode: launch");
    println!("port: {port}");
    println!("headless: {headless}");
    if let Some(path) = chrome_path.as_deref() {
        println!("chrome_path: {path}");
    }

    let mut builder = ChromiumoxideBrowserConfig::builder().port(port).no_sandbox();
    if !headless {
        builder = builder.with_head();
    }
    if let Some(path) = chrome_path.as_deref() {
        builder = builder.chrome_executable(Path::new(path));
    }

    let config = builder
        .build()
        .map_err(|err| format!("failed to build browser config: {err}"))?;
    let (browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|err| format!("failed to launch chrome: {err}"))?;

    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(err) = event {
                eprintln!("handler_error: {err}");
                break;
            }
        }
    });

    let page = browser
        .new_page("about:blank")
        .await
        .map_err(|err| format!("failed to create page: {err}"))?;

    if let Some(url) = url.as_deref() {
        println!("navigate: {url}");
        page.goto(url)
            .await
            .map_err(|err| format!("failed to navigate to {url}: {err}"))?;
    }

    print_pages(&browser).await?;

    if print_cookies {
        print_cookies_for_page(&page, cookie_domain.as_deref()).await?;
    }

    handler_task.abort();
    Ok(())
}

async fn run_ensure_debug_port(
    port: u16,
    timeout_ms: u64,
    chrome_path: Option<String>,
) -> Result<(), String> {
    let config = ToolBrowserConfig {
        port,
        chrome_path,
        profile_dir: None,
        headless: false,
        is_enabled: false,
    };

    let result = debug_port::ensure_debug_port(&config, port, Duration::from_millis(timeout_ms))
        .await?;
    println!("ensure_debug_port: {}", pretty_json(&serde_json::to_value(result).unwrap_or_default()));
    Ok(())
}

async fn print_cookies_for_page(
    page: &chromiumoxide::Page,
    cookie_domain: Option<&str>,
) -> Result<(), String> {
    let cookies = page
        .get_cookies()
        .await
        .map_err(|err| format!("failed to read cookies: {err}"))?;
    let cookies: Vec<Value> = cookies
        .into_iter()
        .filter(|cookie| {
            cookie_domain
                .map(|domain| cookie.domain.contains(domain))
                .unwrap_or(true)
        })
        .map(|cookie| {
            serde_json::json!({
                "name": cookie.name,
                "domain": cookie.domain,
                "path": cookie.path,
                "secure": cookie.secure,
                "http_only": cookie.http_only,
                "same_site": cookie.same_site,
            })
        })
        .collect();
    println!("cookies: {}", pretty_json(&Value::Array(cookies)));
    Ok(())
}

async fn probe_tcp(port: u16) -> bool {
    tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok()
}

async fn fetch_json(client: &reqwest::Client, url: &str) -> Result<Value, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("GET {url} failed: {err}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| format!("reading {url} failed: {err}"))?;
    if !status.is_success() {
        return Err(format!("GET {url} returned {status}: {body}"));
    }
    serde_json::from_str(&body).map_err(|err| format!("invalid json from {url}: {err}"))
}

async fn connect_browser(
    endpoint: &str,
) -> Result<(Browser, tokio::task::JoinHandle<()>), String> {
    let (browser, mut handler) = Browser::connect(endpoint)
        .await
        .map_err(|err| format!("Browser::connect({endpoint}) failed: {err}"))?;

    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(err) = event {
                eprintln!("handler_error: {err}");
                break;
            }
        }
    });

    Ok((browser, handler_task))
}

async fn print_pages(browser: &Browser) -> Result<(), String> {
    let pages = browser
        .pages()
        .await
        .map_err(|err| format!("failed to list pages: {err}"))?;
    println!("page_count: {}", pages.len());

    for (index, page) in pages.iter().enumerate() {
        let url = page
            .url()
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "<unavailable>".to_string());
        println!("page[{index}]: {url}");
    }

    Ok(())
}

fn endpoint_base(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_connect_defaults_to_standard_port() {
        let command = parse_debug_command(["browser_debug", "connect"]).unwrap();
        assert_eq!(
            command,
            DebugCommand::Connect {
                port: 9222,
                url: None,
                print_cookies: false,
                cookie_domain: None,
            }
        );
    }

    #[test]
    fn parse_launch_supports_headless_and_custom_port() {
        let command = parse_debug_command([
            "browser_debug",
            "launch",
            "--port",
            "9333",
            "--headless",
        ])
        .unwrap();

        assert_eq!(
            command,
            DebugCommand::Launch {
                port: 9333,
                headless: true,
                chrome_path: None,
                url: None,
                print_cookies: false,
                cookie_domain: None,
            }
        );
    }

    #[test]
    fn parse_launch_supports_navigation_and_cookie_output() {
        let command = parse_debug_command([
            "browser_debug",
            "launch",
            "--headless",
            "--url",
            "https://github.com",
            "--print-cookies",
            "--cookie-domain",
            "github.com",
        ])
        .unwrap();

        assert_eq!(
            command,
            DebugCommand::Launch {
                port: 9222,
                headless: true,
                chrome_path: None,
                url: Some("https://github.com".to_string()),
                print_cookies: true,
                cookie_domain: Some("github.com".to_string()),
            }
        );
    }

    #[test]
    fn parse_connect_supports_navigation_and_cookie_output() {
        let command = parse_debug_command([
            "browser_debug",
            "connect",
            "--port",
            "9222",
            "--url",
            "https://github.com",
            "--print-cookies",
            "--cookie-domain",
            "github.com",
        ])
        .unwrap();

        assert_eq!(
            command,
            DebugCommand::Connect {
                port: 9222,
                url: Some("https://github.com".to_string()),
                print_cookies: true,
                cookie_domain: Some("github.com".to_string()),
            }
        );
    }

    #[test]
    fn parse_ensure_debug_port_supports_timeout_and_path() {
        let command = parse_debug_command([
            "browser_debug",
            "ensure-debug-port",
            "--port",
            "9222",
            "--timeout-ms",
            "15000",
            "--chrome-path",
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ])
        .unwrap();

        assert_eq!(
            command,
            DebugCommand::EnsureDebugPort {
                port: 9222,
                timeout_ms: 15_000,
                chrome_path: Some(
                    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".to_string()
                ),
            }
        );
    }
}
