import { describe, expect, it } from "vitest";

import { sanitizeAssistantMarkdown } from "./sanitize-assistant-markdown";

describe("sanitizeAssistantMarkdown", () => {
  it("removes bracket style tool_call fragments", () => {
    const input =
      "先看目录\n[tool_call] id=call_1 name=shell args={\"command\":\"ls\"}\n再继续";

    expect(sanitizeAssistantMarkdown(input)).toBe("先看目录\n再继续");
  });

  it("removes xml style tool_call blocks and dangling arg_value tags", () => {
    const input =
      "说明\n<tool_call id=\"call_2\"><arg_value>{\"k\":1}</arg_value></tool_call>\n尾部</arg_value></tool_call>";

    expect(sanitizeAssistantMarkdown(input)).toBe("说明\n尾部");
  });
});

