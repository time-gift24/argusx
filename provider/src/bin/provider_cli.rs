use argus_core::{ResponseEvent, ToolCall};
use futures::StreamExt;
use provider::{Dialect, ProviderClient, ProviderConfig, Request};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("=== Provider Streaming Test CLI ===\n");

    // Select provider
    println!("Select provider:");
    println!("1. OpenAI");
    println!("2. Zai");
    print!("> ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let dialect = match choice.trim() {
        "1" => Dialect::Openai,
        "2" => Dialect::Zai,
        _ => {
            println!("Invalid choice, defaulting to OpenAI");
            Dialect::Openai
        }
    };

    // Enter API key
    print!("Enter API key: ");
    io::stdout().flush()?;
    let api_key = rpassword::read_password().unwrap_or_else(|_| {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap_or_default();
        input.trim().to_string()
    });

    if api_key.is_empty() {
        eprintln!("Error: API key is required");
        std::process::exit(1);
    }

    // Enter base URL
    let default_url = match dialect {
        Dialect::Openai => "https://api.openai.com/v1",
        Dialect::Zai => "https://open.bigmodel.cn/api/coding/paas/v4",
    };

    println!("Enter base URL (default: {}):", default_url);
    print!("> ");
    io::stdout().flush()?;
    let mut base_url = String::new();
    io::stdin().read_line(&mut base_url)?;
    let base_url = if base_url.trim().is_empty() {
        default_url.to_string()
    } else {
        base_url.trim().to_string()
    };

    // Enter model
    let default_model = match dialect {
        Dialect::Openai => "gpt-4o",
        Dialect::Zai => "glm-5",
    };

    println!("Enter model (default: {}):", default_model);
    print!("> ");
    io::stdout().flush()?;
    let mut model = String::new();
    io::stdin().read_line(&mut model)?;
    let model = if model.trim().is_empty() {
        default_model.to_string()
    } else {
        model.trim().to_string()
    };

    // Enter message
    println!("Enter your message:");
    print!("> ");
    io::stdout().flush()?;
    let mut message = String::new();
    io::stdin().read_line(&mut message)?;
    if message.trim().is_empty() {
        eprintln!("Error: message is required");
        std::process::exit(1);
    }
    let message = message.trim().to_string();

    // Build request
    let request = build_request(&model, &message);

    // Build config
    let config = ProviderConfig::new(dialect, base_url, api_key);

    // Create client
    let client = match ProviderClient::new(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creating client: {}", e);
            std::process::exit(1);
        }
    };

    // Stream request
    let stream = match client.stream(request) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error starting stream: {}", e);
            std::process::exit(1);
        }
    };

    println!("\n--- Streaming Response ---\n");

    // Process stream
    let mut stream = stream;
    while let Some(event) = stream.next().await {
        match event {
            ResponseEvent::ContentDelta(text) => {
                print!("{}", text);
                io::stdout().flush().ok();
            }
            ResponseEvent::ReasoningDelta(text) => {
                print!("[reasoning: {}]", text);
                io::stdout().flush().ok();
            }
            ResponseEvent::ContentDone(text) => {
                println!("\n[content done: {}]", text);
            }
            ResponseEvent::ReasoningDone(text) => {
                println!("[reasoning done: {}]", text);
            }
            ResponseEvent::Done(usage) => {
                println!("\n--- Done ---");
                if let Some(usage) = usage {
                    println!("Usage: {:?}", usage);
                }
            }
            ResponseEvent::Error(err) => {
                println!("\nError: {}", err.message);
            }
            ResponseEvent::Created(meta) => {
                println!("[created: id={}]", meta.id);
            }
            ResponseEvent::ToolDelta(text) => {
                print!("[tool: {}]", text);
                io::stdout().flush().ok();
            }
            ResponseEvent::ToolDone(call) => {
                let name = match call {
                    ToolCall::FunctionCall { name, .. } => name.clone(),
                    ToolCall::Builtin(call) => call.builtin.canonical_name().to_string(),
                    ToolCall::Mcp(mcp) => mcp.name.clone().unwrap_or_default(),
                };
                println!("[tool done: {}]", name);
            }
        }
    }

    Ok(())
}

fn build_request(model: &str, message: &str) -> Request {
    use provider::dialect::openai::schema::common::Role;
    use provider::dialect::openai::schema::request::ChatMessage;

    provider::dialect::openai::schema::request::ChatCompletionsOptions {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: Role::User,
            content: Some(message.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            extra: Default::default(),
        }],
        stream: Some(true),
        ..Default::default()
    }
}
