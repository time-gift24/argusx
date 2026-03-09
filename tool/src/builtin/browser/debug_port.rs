use chromiumoxide::Browser;
use futures::StreamExt;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::config::BrowserConfig;

pub const DEFAULT_DEBUG_PORT: u16 = 9222;
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const SNAPSHOT_FIELD_DELIMITER: &str = "|||";

#[derive(Debug, Clone, Serialize)]
pub struct EnsureDebugPortResult {
    pub port: u16,
    pub already_enabled: bool,
    pub restarted: bool,
    pub captured_window_count: usize,
    pub captured_tab_count: usize,
    pub restored_tab_count: usize,
    pub skipped_tab_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSessionSnapshot {
    pub windows: Vec<BrowserWindowSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserWindowSnapshot {
    pub active_tab_index: usize,
    pub tabs: Vec<BrowserTabSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserTabSnapshot {
    pub url: String,
    pub kind: BrowserTabKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserTabKind {
    Ordinary,
    ChromeInternal,
    Extension,
    Unknown,
}

pub async fn ensure_debug_port(
    config: &BrowserConfig,
    port: u16,
    timeout: Duration,
) -> Result<EnsureDebugPortResult, String> {
    if probe_debug_port(port).await? {
        return Ok(EnsureDebugPortResult {
            port,
            already_enabled: true,
            restarted: false,
            captured_window_count: 0,
            captured_tab_count: 0,
            restored_tab_count: 0,
            skipped_tab_count: 0,
            warnings: vec![],
        });
    }

    let mut warnings = Vec::new();
    if config.profile_dir.is_none() {
        warnings.push(format!(
            "using a non-standard user-data-dir because Chrome 136+ ignores --remote-debugging-port on the default profile"
        ));
    }
    let snapshot = capture_session_snapshot().await.map_err(|err| {
        format!("failed to capture browser session before restart: {err}")
    })?;

    if snapshot.windows.is_empty() {
        warnings.push("no browser tabs captured before restart".to_string());
    }

    quit_browser().await?;
    relaunch_browser(config, port).await?;
    wait_for_debug_port(port, timeout).await?;

    let restored_tab_count = restore_missing_tabs(port, &snapshot, &mut warnings).await?;
    let captured_tab_count = snapshot.windows.iter().map(|window| window.tabs.len()).sum();
    let skipped_tab_count = snapshot
        .windows
        .iter()
        .flat_map(|window| window.tabs.iter())
        .filter(|tab| tab.url.is_empty())
        .count();

    Ok(EnsureDebugPortResult {
        port,
        already_enabled: false,
        restarted: true,
        captured_window_count: snapshot.windows.len(),
        captured_tab_count,
        restored_tab_count,
        skipped_tab_count,
        warnings,
    })
}

pub async fn probe_debug_port(port: u16) -> Result<bool, String> {
    if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_err() {
        return Ok(false);
    }

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/json/version"))
        .send()
        .await
        .map_err(|err| format!("probe request failed: {err}"))?;

    Ok(response.status().is_success())
}

async fn wait_for_debug_port(port: u16, timeout: Duration) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if probe_debug_port(port).await? {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for Chrome debug port {port} to become available"
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn capture_session_snapshot() -> Result<BrowserSessionSnapshot, String> {
    #[cfg(target_os = "macos")]
    {
        capture_session_snapshot_macos().await
    }

    #[cfg(target_os = "windows")]
    {
        capture_session_snapshot_windows().await
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("ensure_debug_port is only supported on macOS and Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
async fn capture_session_snapshot_windows() -> Result<BrowserSessionSnapshot, String> {
    let output = run_powershell(&build_windows_capture_script()).await?;
    Ok(parse_windows_snapshot(&output))
}

#[cfg(target_os = "macos")]
async fn capture_session_snapshot_macos() -> Result<BrowserSessionSnapshot, String> {
    let script = format!(
        r#"
tell application "Google Chrome"
    set output to ""
    repeat with windowIndex from 1 to count of windows
        set w to window windowIndex
        set activeIndex to active tab index of w
        repeat with tabIndex from 1 to count of tabs of w
            set t to tab tabIndex of w
            set tabUrl to URL of t
            set output to output & windowIndex & "{delimiter}" & tabIndex & "{delimiter}" & activeIndex & "{delimiter}" & tabUrl & linefeed
        end repeat
    end repeat
    return output
end tell
"#,
        delimiter = SNAPSHOT_FIELD_DELIMITER
    );

    let output = run_osascript(&script).await?;
    Ok(parse_macos_snapshot(&output))
}

#[cfg(target_os = "macos")]
async fn run_osascript(script: &str) -> Result<String, String> {
    let mut child = Command::new("osascript")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to invoke osascript: {err}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open osascript stdin".to_string())?;
    stdin
        .write_all(script.as_bytes())
        .await
        .map_err(|err| format!("failed to write osascript script: {err}"))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|err| format!("failed to wait for osascript: {err}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_macos_snapshot(output: &str) -> BrowserSessionSnapshot {
    let mut windows: Vec<BrowserWindowSnapshot> = Vec::new();

    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.splitn(4, SNAPSHOT_FIELD_DELIMITER);
        let window_index = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        let _tab_index = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        let active_index = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        let url = parts.next().unwrap_or_default().trim().to_string();

        while windows.len() < window_index {
            windows.push(BrowserWindowSnapshot {
                active_tab_index: 0,
                tabs: Vec::new(),
            });
        }

        let window = &mut windows[window_index - 1];
        window.active_tab_index = active_index.saturating_sub(1);
        window.tabs.push(BrowserTabSnapshot {
            kind: classify_tab_kind(&url),
            url,
        });
    }

    BrowserSessionSnapshot { windows }
}

async fn quit_browser() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        run_osascript(r#"tell application "Google Chrome" to quit"#).await?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        run_powershell(&build_windows_stop_script()).await.map(|_| ())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("unsupported platform".to_string())
    }
}

async fn relaunch_browser(config: &BrowserConfig, port: u16) -> Result<(), String> {
    let user_data_dir = debug_user_data_dir(config, port);

    #[cfg(target_os = "macos")]
    {
        if let Some(path) = config.chrome_path.as_deref() {
            Command::new(path)
                .args([
                    &format!("--remote-debugging-port={port}"),
                    &format!("--user-data-dir={}", user_data_dir.display()),
                    "--restore-last-session",
                    "about:blank",
                ])
                .spawn()
                .map_err(|err| format!("failed to relaunch Chrome: {err}"))?;
        } else {
            Command::new("open")
                .args([
                    "-na",
                    "Google Chrome",
                    "--args",
                    &format!("--remote-debugging-port={port}"),
                    &format!("--user-data-dir={}", user_data_dir.display()),
                    "--restore-last-session",
                    "about:blank",
                ])
                .spawn()
                .map_err(|err| format!("failed to relaunch Chrome: {err}"))?;
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let file_path = config
            .chrome_path
            .clone()
            .unwrap_or_else(|| "chrome".to_string());
        run_powershell(&build_windows_relaunch_script(
            &file_path,
            &user_data_dir,
            port,
        ))
        .await
        .map(|_| ())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (config, port);
        Err("unsupported platform".to_string())
    }
}

fn debug_user_data_dir(config: &BrowserConfig, port: u16) -> PathBuf {
    if let Some(profile_dir) = config.profile_dir.as_deref() {
        return PathBuf::from(profile_dir);
    }

    std::env::temp_dir().join(format!("argusx-chrome-debug-profile-{port}"))
}

#[cfg(target_os = "windows")]
async fn run_powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .await
        .map_err(|err| format!("failed to invoke powershell: {err}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(any(test, target_os = "windows"))]
fn parse_windows_snapshot(output: &str) -> BrowserSessionSnapshot {
    let mut windows = Vec::new();

    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.splitn(4, SNAPSHOT_FIELD_DELIMITER);
        let window_index = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        let _tab_index = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        let active_index = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        let url = parts.next().unwrap_or_default().trim().to_string();

        while windows.len() < window_index {
            windows.push(BrowserWindowSnapshot {
                active_tab_index: 0,
                tabs: Vec::new(),
            });
        }

        let window = &mut windows[window_index - 1];
        window.active_tab_index = active_index.saturating_sub(1);
        if !url.is_empty() {
            window.tabs.push(BrowserTabSnapshot {
                kind: classify_tab_kind(&url),
                url,
            });
        }
    }

    BrowserSessionSnapshot { windows }
}

#[cfg(any(test, target_os = "windows"))]
fn build_windows_capture_script() -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::RootElement
$windowIndex = 0
Get-Process chrome -ErrorAction SilentlyContinue |
    Where-Object {{ $_.MainWindowHandle -ne 0 }} |
    ForEach-Object {{
        $windowIndex += 1
        $window = [System.Windows.Automation.AutomationElement]::FromHandle($_.MainWindowHandle)
        if ($null -eq $window) {{ return }}
        $edits = $window.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Edit
            ))
        )

        $url = $null
        foreach ($edit in $edits) {{
            try {{
                $pattern = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
                if ($pattern -and $pattern.Current.Value -match '^(https?|chrome|edge)://') {{
                    $url = $pattern.Current.Value
                    break
                }}
            }} catch {{
            }}
        }}

        if ($url) {{
            Write-Output ($windowIndex.ToString() + '{delimiter}' + '1' + '{delimiter}' + '1' + '{delimiter}' + $url)
        }}
    }}
"#,
        delimiter = SNAPSHOT_FIELD_DELIMITER
    )
}

#[cfg(any(test, target_os = "windows"))]
fn build_windows_stop_script() -> String {
    "Stop-Process -Name chrome -Force -ErrorAction SilentlyContinue".to_string()
}

#[cfg(any(test, target_os = "windows"))]
fn build_windows_relaunch_script(
    file_path: &str,
    user_data_dir: &std::path::Path,
    port: u16,
) -> String {
    format!(
        "Start-Process -FilePath '{}' -ArgumentList '--remote-debugging-port={port}','--user-data-dir={}','--restore-last-session','about:blank'",
        file_path.replace('\'', "''"),
        user_data_dir.display().to_string().replace('\'', "''")
    )
}

async fn restore_missing_tabs(
    port: u16,
    snapshot: &BrowserSessionSnapshot,
    warnings: &mut Vec<String>,
) -> Result<usize, String> {
    let existing_urls = fetch_current_page_urls(port).await?;
    let desired_urls = collect_restore_urls(snapshot);
    let missing_urls: Vec<String> = desired_urls
        .into_iter()
        .filter(|url| !existing_urls.contains(url))
        .collect();

    if missing_urls.is_empty() {
        return Ok(0);
    }

    let endpoint = format!("http://127.0.0.1:{port}");
    let (browser, mut handler) = Browser::connect(&endpoint)
        .await
        .map_err(|err| format!("failed to connect to relaunched Chrome: {err}"))?;
    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(err) = event {
                tracing::warn!("browser restore handler stopped: {err}");
                break;
            }
        }
    });

    let mut restored = 0usize;
    for url in missing_urls {
        match browser.new_page(url.as_str()).await {
            Ok(_) => restored += 1,
            Err(err) => warnings.push(format!("failed to restore tab {url}: {err}")),
        }
    }

    handler_task.abort();
    Ok(restored)
}

async fn fetch_current_page_urls(port: u16) -> Result<BTreeSet<String>, String> {
    let targets: Vec<Value> = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/json/list"))
        .send()
        .await
        .map_err(|err| format!("failed to query current Chrome targets: {err}"))?
        .json()
        .await
        .map_err(|err| format!("failed to decode current Chrome targets: {err}"))?;

    Ok(targets
        .into_iter()
        .filter(|target| target["type"].as_str() == Some("page"))
        .filter_map(|target| target["url"].as_str().map(str::to_string))
        .collect())
}

fn collect_restore_urls(snapshot: &BrowserSessionSnapshot) -> BTreeSet<String> {
    snapshot
        .windows
        .iter()
        .flat_map(|window| window.tabs.iter())
        .filter_map(|tab| {
            if tab.url.is_empty() {
                None
            } else {
                Some(tab.url.clone())
            }
        })
        .collect()
}

fn classify_tab_kind(url: &str) -> BrowserTabKind {
    if url.starts_with("http://") || url.starts_with("https://") {
        BrowserTabKind::Ordinary
    } else if url.starts_with("chrome://") {
        BrowserTabKind::ChromeInternal
    } else if url.starts_with("chrome-extension://") {
        BrowserTabKind::Extension
    } else {
        BrowserTabKind::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_macos_snapshot_groups_tabs_by_window() {
        let snapshot = parse_macos_snapshot(
            "1|||1|||2|||https://example.com\n1|||2|||2|||chrome://settings/\n2|||1|||1|||chrome-extension://abc/page.html\n",
        );

        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.windows[0].active_tab_index, 1);
        assert_eq!(snapshot.windows[0].tabs.len(), 2);
        assert_eq!(snapshot.windows[1].tabs[0].kind, BrowserTabKind::Extension);
    }

    #[test]
    fn collect_restore_urls_deduplicates_urls() {
        let snapshot = BrowserSessionSnapshot {
            windows: vec![BrowserWindowSnapshot {
                active_tab_index: 0,
                tabs: vec![
                    BrowserTabSnapshot {
                        url: "https://example.com".to_string(),
                        kind: BrowserTabKind::Ordinary,
                    },
                    BrowserTabSnapshot {
                        url: "https://example.com".to_string(),
                        kind: BrowserTabKind::Ordinary,
                    },
                ],
            }],
        };

        let urls = collect_restore_urls(&snapshot);
        assert_eq!(urls.len(), 1);
        assert!(urls.contains("https://example.com"));
    }

    #[test]
    fn classify_tab_kind_recognizes_special_urls() {
        assert_eq!(classify_tab_kind("https://github.com"), BrowserTabKind::Ordinary);
        assert_eq!(classify_tab_kind("chrome://settings"), BrowserTabKind::ChromeInternal);
        assert_eq!(
            classify_tab_kind("chrome-extension://abc/page.html"),
            BrowserTabKind::Extension
        );
    }

    #[test]
    fn debug_user_data_dir_defaults_to_non_standard_temp_dir() {
        let config = BrowserConfig::default();
        let path = debug_user_data_dir(&config, 9222);
        assert!(path.to_string_lossy().contains("argusx-chrome-debug-profile-9222"));
    }

    #[test]
    fn parse_windows_snapshot_keeps_active_tab_url_per_window() {
        let snapshot = parse_windows_snapshot(
            "1|||1|||1|||https://example.com\n2|||1|||1|||chrome://settings/\n",
        );

        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.windows[0].tabs[0].url, "https://example.com");
        assert_eq!(snapshot.windows[1].tabs[0].kind, BrowserTabKind::ChromeInternal);
    }

    #[test]
    fn windows_capture_script_targets_ui_automation_and_urls() {
        let script = build_windows_capture_script();
        assert!(script.contains("UIAutomationClient"));
        assert!(script.contains("Get-Process chrome"));
        assert!(script.contains(SNAPSHOT_FIELD_DELIMITER));
        assert!(script.contains("https?|chrome|edge"));
    }

    #[test]
    fn windows_relaunch_script_includes_debug_port_and_profile() {
        let script = build_windows_relaunch_script(
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
            std::path::Path::new("C:\\temp\\argusx-profile"),
            9222,
        );

        assert!(script.contains("--remote-debugging-port=9222"));
        assert!(script.contains("--user-data-dir=C:\\temp\\argusx-profile"));
        assert!(script.contains("--restore-last-session"));
    }

    #[test]
    fn windows_stop_script_stops_chrome_processes() {
        let script = build_windows_stop_script();
        assert!(script.contains("Stop-Process"));
        assert!(script.contains("chrome"));
    }
}
