const CODE_HIGHLIGHT_ALLOWLIST = new Set([
  "rust",
  "typescript",
  "ts",
  "javascript",
  "js",
  "tsx",
  "jsx",
  "python",
  "py",
  "go",
  "java",
  "bash",
  "shell",
  "shell-session",
  "json",
  "yaml",
  "yml",
  "toml",
  "sql",
  "html",
  "css",
]);

export function shouldHighlightFence(input: {
  isFenced: boolean;
  language?: string;
}): boolean {
  if (!input.isFenced) return false;
  const lang = (input.language ?? "").trim().toLowerCase();
  if (!lang || lang === "text") return false;
  return CODE_HIGHLIGHT_ALLOWLIST.has(lang);
}
