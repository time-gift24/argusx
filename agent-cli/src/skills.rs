use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub location: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct SkillCatalog {
    entries: Vec<SkillEntry>,
    by_name: HashMap<String, usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillInjection {
    pub message: String,
    pub applied: Vec<String>,
    pub warnings: Vec<String>,
}

impl SkillCatalog {
    pub fn discover(cwd: &Path) -> Self {
        let mut roots = vec![
            cwd.join(".agents").join("skills"),
            cwd.join(".codex").join("skills"),
            cwd.join(".claude").join("skills"),
        ];
        if let Some(home) = home_dir() {
            roots.push(home.join(".agents").join("skills"));
            roots.push(home.join(".codex").join("skills"));
            roots.push(home.join(".codex").join("superpowers").join("skills"));
        }
        Self::from_roots(&roots)
    }

    pub fn from_roots(roots: &[PathBuf]) -> Self {
        let mut entries = Vec::new();
        let mut by_name = HashMap::new();

        for root in roots {
            let mut files = Vec::new();
            if collect_skill_files(root, &mut files).is_err() {
                continue;
            }
            files.sort();
            for skill_file in files {
                let Ok(loaded) = read_loaded_skill(&skill_file) else {
                    continue;
                };
                let key = loaded.entry.name.to_lowercase();
                if by_name.contains_key(&key) {
                    continue;
                }
                by_name.insert(key, entries.len());
                entries.push(loaded.entry);
            }
        }

        Self { entries, by_name }
    }

    pub fn entries(&self) -> &[SkillEntry] {
        &self.entries
    }

    pub fn compose_system_prompt(&self, base: Option<String>) -> Option<String> {
        if self.entries.is_empty() {
            return base;
        }

        let mut section = String::new();
        section.push_str("## Skills\n");
        section.push_str("A skill is a set of local instructions stored in a `SKILL.md` file.\n");
        section.push_str("### Available skills\n");
        for entry in &self.entries {
            section.push_str(&format!(
                "- {}: {} (file: {})\n",
                entry.name,
                entry.description,
                entry.location.display()
            ));
        }
        section.push_str("### How to use skills\n");
        section.push_str(
            "- Trigger rules: if user explicitly names a skill (like `$name` or a SKILL.md link), you must use it.\n",
        );
        section.push_str(
            "- Load only the `SKILL.md` and only the extra referenced files needed for the current request.\n",
        );
        section.push_str(
            "- Prefer minimal skill set: if multiple skills match, use the smallest set that fully covers the task.\n",
        );

        match base {
            Some(existing) if !existing.trim().is_empty() => {
                Some(format!("{existing}\n\n{section}"))
            }
            _ => Some(section),
        }
    }

    pub fn inject_user_message(&self, message: &str) -> SkillInjection {
        let mut warnings = Vec::new();
        let mut selected = Vec::new();
        let mut selected_keys = HashSet::new();
        let mut applied = Vec::new();

        for path_mention in extract_markdown_skill_links(message)
            .into_iter()
            .chain(extract_bare_skill_paths(message))
        {
            let Some(path) = resolve_path_mention(&path_mention) else {
                warnings.push(format!("unresolved skill path: {path_mention}"));
                continue;
            };
            let key = path.to_string_lossy().to_string();
            if selected_keys.contains(&key) {
                continue;
            }
            match read_loaded_skill(&path) {
                Ok(skill) => {
                    selected_keys.insert(key);
                    applied.push(skill.entry.name.clone());
                    selected.push(skill);
                }
                Err(err) => {
                    warnings.push(format!("failed to load skill {}: {err}", path.display()));
                }
            }
        }

        for name in extract_dollar_mentions(message) {
            let key = name.to_lowercase();
            let Some(index) = self.by_name.get(&key).copied() else {
                warnings.push(format!("skill not found: {name}"));
                continue;
            };
            let entry = &self.entries[index];
            let path_key = entry.location.to_string_lossy().to_string();
            if selected_keys.contains(&path_key) {
                continue;
            }
            match read_loaded_skill(&entry.location) {
                Ok(skill) => {
                    selected_keys.insert(path_key);
                    applied.push(skill.entry.name.clone());
                    selected.push(skill);
                }
                Err(err) => {
                    warnings.push(format!(
                        "failed to load skill {}: {err}",
                        entry.location.display()
                    ));
                }
            }
        }

        if selected.is_empty() {
            return SkillInjection {
                message: message.to_string(),
                applied,
                warnings,
            };
        }

        let mut out = String::new();
        out.push_str("<skills>\n");
        for skill in selected {
            out.push_str(&format!(
                "<skill name=\"{}\" path=\"{}\">\n{}\n</skill>\n",
                skill.entry.name,
                skill.entry.location.display(),
                skill.content.trim()
            ));
        }
        out.push_str("</skills>\n\n");
        out.push_str(message);

        SkillInjection {
            message: out,
            applied,
            warnings,
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedSkill {
    entry: SkillEntry,
    content: String,
}

fn collect_skill_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    if root.is_file() {
        if root
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        {
            out.push(root.to_path_buf());
        }
        return Ok(());
    }
    if !root.is_dir() {
        return Ok(());
    }

    for item in fs::read_dir(root)? {
        let item = item?;
        let path = item.path();
        let file_type = item.file_type()?;
        if file_type.is_dir() {
            collect_skill_files(&path, out)?;
            continue;
        }
        if file_type.is_file()
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn read_loaded_skill(path: &Path) -> io::Result<LoadedSkill> {
    let content = fs::read_to_string(path)?;
    let (fm_name, fm_desc) = parse_frontmatter(&content);

    let name = fm_name.unwrap_or_else(|| {
        path.parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("unknown-skill")
            .to_string()
    });

    let description = fm_desc.unwrap_or_else(|| infer_description(&content));

    Ok(LoadedSkill {
        entry: SkillEntry {
            name,
            description,
            location: path.to_path_buf(),
        },
        content,
    })
}

fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>) {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return (None, None);
    };
    if first.trim() != "---" {
        return (None, None);
    }

    let mut name = None;
    let mut description = None;
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = strip_quotes(value.trim());
        if key == "name" {
            name = Some(value.to_string());
        } else if key == "description" {
            description = Some(value.to_string());
        }
    }
    (name, description)
}

fn infer_description(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "---" {
            continue;
        }
        if trimmed.starts_with("name:") || trimmed.starts_with("description:") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return rest.trim().to_string();
        }
        return trimmed.to_string();
    }
    "No description".to_string()
}

fn strip_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn extract_dollar_mentions(message: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    let chars: Vec<char> = message.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] != '$' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < chars.len() {
            let c = chars[i];
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                i += 1;
            } else {
                break;
            }
        }
        if i > start {
            let name: String = chars[start..i].iter().collect();
            let key = name.to_lowercase();
            if seen.insert(key) {
                names.push(name);
            }
        }
    }

    names
}

fn extract_markdown_skill_links(message: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut index = 0usize;
    while let Some(rel) = message[index..].find("](") {
        let start = index + rel + 2;
        let Some(end_rel) = message[start..].find(')') else {
            break;
        };
        let end = start + end_rel;
        let raw = message[start..end].trim();
        if raw.to_ascii_lowercase().contains("skill.md") {
            paths.push(raw.to_string());
        }
        index = end + 1;
    }
    dedupe_strings(paths)
}

fn extract_bare_skill_paths(message: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for token in message.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| {
            c == '"'
                || c == '\''
                || c == ','
                || c == '.'
                || c == ';'
                || c == '('
                || c == ')'
                || c == '['
                || c == ']'
        });
        if trimmed.to_ascii_lowercase().contains("skill.md") {
            paths.push(trimmed.to_string());
        }
    }
    dedupe_strings(paths)
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let key = value.to_lowercase();
        if seen.insert(key) {
            out.push(value);
        }
    }
    out
}

fn resolve_path_mention(raw: &str) -> Option<PathBuf> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return None;
    }
    if let Some(path) = candidate.strip_prefix("file://") {
        return Some(PathBuf::from(path));
    }
    if candidate.starts_with("~/") {
        let home = home_dir()?;
        return Some(home.join(candidate.trim_start_matches("~/")));
    }
    let path = PathBuf::from(candidate);
    if path.is_absolute() {
        return Some(path);
    }
    if candidate.contains("://") {
        return None;
    }
    let cwd = std::env::current_dir().ok()?;
    Some(cwd.join(path))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn discover_from_roots_loads_skills() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("skills");
        fs::create_dir_all(root.join("brainstorming")).expect("create brainstorming dir");
        fs::create_dir_all(root.join("plain-skill")).expect("create plain-skill dir");

        fs::write(
            root.join("brainstorming").join("SKILL.md"),
            r#"---
name: brainstorming
description: "Design first before implementation"
---

# Brainstorming
"#,
        )
        .expect("write brainstorming skill");

        fs::write(
            root.join("plain-skill").join("SKILL.md"),
            "# Plain\n\nNo frontmatter.",
        )
        .expect("write plain skill");

        let catalog = SkillCatalog::from_roots(&[root]);
        let names = catalog
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();

        assert!(
            names.contains(&"brainstorming"),
            "brainstorming should be discovered"
        );
        assert!(
            names.contains(&"plain-skill"),
            "fallback name should use folder name"
        );

        let brainstorming = catalog
            .entries()
            .iter()
            .find(|entry| entry.name == "brainstorming")
            .expect("brainstorming should exist");
        assert_eq!(
            brainstorming.description,
            "Design first before implementation"
        );
    }

    #[test]
    fn compose_system_prompt_includes_catalog_and_rules() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("skills");
        fs::create_dir_all(root.join("brainstorming")).expect("create dir");
        fs::write(
            root.join("brainstorming").join("SKILL.md"),
            r#"---
name: brainstorming
description: "Design first before implementation"
---
"#,
        )
        .expect("write skill");

        let catalog = SkillCatalog::from_roots(&[root]);
        let prompt = catalog
            .compose_system_prompt(Some("base system".to_string()))
            .expect("prompt should exist");

        assert!(prompt.contains("base system"));
        assert!(prompt.contains("## Skills"));
        assert!(prompt.contains("### Available skills"));
        assert!(prompt.contains("brainstorming"));
        assert!(prompt.contains("Design first before implementation"));
        assert!(prompt.contains("### How to use skills"));
    }

    #[test]
    fn inject_user_message_by_dollar_name_includes_skill_content() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("skills");
        fs::create_dir_all(root.join("brainstorming")).expect("create dir");
        fs::write(
            root.join("brainstorming").join("SKILL.md"),
            r#"---
name: brainstorming
description: "Design first before implementation"
---

# Brainstorming
Follow design workflow.
"#,
        )
        .expect("write skill");

        let catalog = SkillCatalog::from_roots(&[root]);
        let out = catalog.inject_user_message("请使用 $brainstorming 先做设计");

        assert!(
            out.message.contains("<skill name=\"brainstorming\""),
            "message should include injected skill block"
        );
        assert!(
            out.message.contains("Follow design workflow."),
            "message should include full skill content"
        );
        assert_eq!(out.applied, vec!["brainstorming".to_string()]);
    }

    #[test]
    fn inject_user_message_by_markdown_link_includes_skill_content() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("skills");
        fs::create_dir_all(root.join("brainstorming")).expect("create dir");
        let skill_path = root.join("brainstorming").join("SKILL.md");
        fs::write(
            &skill_path,
            r#"---
name: brainstorming
description: "Design first before implementation"
---

Use this before coding.
"#,
        )
        .expect("write skill");

        let catalog = SkillCatalog::from_roots(&[root]);
        let message = format!(
            "[${}](/{}) 为项目提供设计",
            "brainstorming",
            skill_path
                .to_string_lossy()
                .trim_start_matches(std::path::MAIN_SEPARATOR)
        );
        let out = catalog.inject_user_message(&message);

        assert!(out.message.contains("<skill name=\"brainstorming\""));
        assert!(out.message.contains("Use this before coding."));
        assert_eq!(out.applied, vec!["brainstorming".to_string()]);
    }

    #[test]
    fn inject_user_message_without_mentions_is_passthrough() {
        let catalog = SkillCatalog::default();
        let raw = "普通消息";
        let out = catalog.inject_user_message(raw);
        assert_eq!(out.message, raw);
        assert!(out.applied.is_empty());
        assert!(out.warnings.is_empty());
    }
}
