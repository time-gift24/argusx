import { CHAT_MARKDOWN_CLASS } from "./chat-markdown-class";

describe("CHAT_MARKDOWN_CLASS", () => {
  it("forces list markers to align with markdown content start", () => {
    expect(CHAT_MARKDOWN_CLASS).toContain("[&_ul]:list-inside");
    expect(CHAT_MARKDOWN_CLASS).toContain("[&_ol]:list-inside");
    expect(CHAT_MARKDOWN_CLASS).toContain("[&_ul]:pl-0");
    expect(CHAT_MARKDOWN_CLASS).toContain("[&_ol]:pl-0");
  });
});
