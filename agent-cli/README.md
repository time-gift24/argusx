# agent-cli

Terminal chat UI for the agent facade, powered by ratatui.

## Usage

### New session
```bash
cargo run -p agent-cli -- --api-key $BIGMODEL_API_KEY
```

### Resume session
```bash
cargo run -p agent-cli -- --api-key $BIGMODEL_API_KEY --session <session_id>
```

## Keyboard shortcuts

- `Enter`: Send message
- `Tab`: Toggle reasoning visibility
- `Esc` or `Ctrl+C`: Quit
- `Backspace`: Delete character
- `Char keys`: Type input

## Options

- `--api-key`: BigModel API key (env: BIGMODEL_API_KEY)
- `--base-url`: API base URL (env: BIGMODEL_BASE_URL, required)
- `--model`: Model name (default: glm-5)
- `--system-prompt`: Optional system prompt
- `--session`: Resume existing session by ID
- `--store-dir`: Directory for session storage
- `--debug-events`: Enable debug event logging
