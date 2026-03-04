import { readFileSync } from "node:fs";
import path from "node:path";

describe("desktop/app/styles/streamdown.css", () => {
  it("keeps markdown list markers aligned with paragraph start", () => {
    const cssPath = path.resolve(__dirname, "streamdown.css");
    const css = readFileSync(cssPath, "utf8");

    expect(css).toContain(
      '.llm-chat-markdown [data-streamdown="unordered-list"]'
    );
    expect(css).toContain(
      '.llm-chat-markdown [data-streamdown="ordered-list"]'
    );
    expect(css).toContain("list-style-position: inside !important;");
    expect(css).toContain("padding-inline-start: 0 !important;");
  });
});
