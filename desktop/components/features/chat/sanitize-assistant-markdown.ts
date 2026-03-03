const TOOL_CALL_BLOCK_RE = /<tool_call\b[\s\S]*?<\/tool_call>/gi;
const TOOL_CALL_INLINE_RE = /\[tool_call\][^\n\r]*/gi;
const TOOL_CALL_TAG_RE = /<\/?tool_call\b[^>]*>/gi;
const ARG_VALUE_TAG_RE = /<\/?arg_value\b[^>]*>/gi;

export const sanitizeAssistantMarkdown = (value: string): string => {
  if (!value) {
    return "";
  }

  let sanitized = value;
  sanitized = sanitized.replace(TOOL_CALL_BLOCK_RE, "");
  sanitized = sanitized.replace(TOOL_CALL_INLINE_RE, "");
  sanitized = sanitized.replace(TOOL_CALL_TAG_RE, "");
  sanitized = sanitized.replace(ARG_VALUE_TAG_RE, "");
  sanitized = sanitized.replace(/[ \t]+\n/g, "\n");
  sanitized = sanitized.replace(/\n{2,}/g, "\n");

  return sanitized.trim();
};
