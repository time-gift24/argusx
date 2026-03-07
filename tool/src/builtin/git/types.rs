use serde::Deserialize;
use serde_json::{Map, Value, json};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum GitArgs {
    Status {
        repo_path: String,
        #[serde(default)]
        include_untracked: bool,
    },
    Diff {
        repo_path: String,
        #[serde(default)]
        staged: bool,
        #[serde(default)]
        revision_range: Option<String>,
        #[serde(default)]
        paths: Vec<String>,
        #[serde(default = "default_max_bytes")]
        max_bytes: usize,
    },
    Log {
        repo_path: String,
        #[serde(default = "default_max_count")]
        max_count: usize,
        #[serde(default)]
        revision_range: Option<String>,
        #[serde(default)]
        oneline: bool,
    },
    Show {
        repo_path: String,
        object: String,
        #[serde(default = "default_max_bytes")]
        max_bytes: usize,
    },
    BranchList {
        repo_path: String,
    },
    RemoteList {
        repo_path: String,
    },
    WorktreeList {
        repo_path: String,
    },
    Add {
        repo_path: String,
        paths: Vec<String>,
    },
    Commit {
        repo_path: String,
        message: String,
        #[serde(default)]
        allow_empty: bool,
    },
    BranchCreate {
        repo_path: String,
        branch: String,
        #[serde(default)]
        start_point: Option<String>,
        #[serde(default)]
        checkout: bool,
    },
    Checkout {
        repo_path: String,
        branch: String,
    },
    Clone {
        url: String,
        target_path: String,
        #[serde(default)]
        branch: Option<String>,
    },
    Fetch {
        repo_path: String,
        #[serde(default = "default_remote")]
        remote: String,
        #[serde(default)]
        prune: bool,
    },
}

pub fn default_max_bytes() -> usize {
    65536
}

pub fn default_max_count() -> usize {
    20
}

pub fn default_remote() -> String {
    "origin".to_string()
}

pub fn build_schema() -> Value {
    let one_of: Vec<Value> = vec![
        build_action_schema(
            "status",
            &[
                ("repo_path", "string", true),
                ("include_untracked", "boolean", false),
            ],
        ),
        build_action_schema(
            "diff",
            &[
                ("repo_path", "string", true),
                ("staged", "boolean", false),
                ("revision_range", "string", false),
                ("paths", "array", false),
                ("max_bytes", "integer", false),
            ],
        ),
        build_action_schema(
            "log",
            &[
                ("repo_path", "string", true),
                ("max_count", "integer", false),
                ("revision_range", "string", false),
                ("oneline", "boolean", false),
            ],
        ),
        build_action_schema(
            "show",
            &[
                ("repo_path", "string", true),
                ("object", "string", true),
                ("max_bytes", "integer", false),
            ],
        ),
        build_action_schema("branch_list", &[("repo_path", "string", true)]),
        build_action_schema("remote_list", &[("repo_path", "string", true)]),
        build_action_schema("worktree_list", &[("repo_path", "string", true)]),
        build_action_schema(
            "add",
            &[("repo_path", "string", true), ("paths", "array", true)],
        ),
        build_action_schema(
            "commit",
            &[
                ("repo_path", "string", true),
                ("message", "string", true),
                ("allow_empty", "boolean", false),
            ],
        ),
        build_action_schema(
            "branch_create",
            &[
                ("repo_path", "string", true),
                ("branch", "string", true),
                ("start_point", "string", false),
                ("checkout", "boolean", false),
            ],
        ),
        build_action_schema(
            "checkout",
            &[("repo_path", "string", true), ("branch", "string", true)],
        ),
        build_action_schema(
            "clone",
            &[
                ("url", "string", true),
                ("target_path", "string", true),
                ("branch", "string", false),
            ],
        ),
        build_action_schema(
            "fetch",
            &[
                ("repo_path", "string", true),
                ("remote", "string", false),
                ("prune", "boolean", false),
            ],
        ),
    ];

    json!({ "oneOf": one_of })
}

fn build_action_schema(action: &str, props: &[(&str, &str, bool)]) -> Value {
    let mut properties = Map::new();
    properties.insert("action".to_string(), json!({ "const": action }));

    let mut required = vec!["action".to_string()];

    for (name, typ, is_required) in props {
        let type_schema = match *typ {
            "array" => json!({ "type": "array", "items": { "type": "string" } }),
            _ => json!({ "type": typ }),
        };
        properties.insert(name.to_string(), type_schema);

        if *is_required {
            required.push(name.to_string());
        }
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}
