import type { HighlightOptions, HighlightResult } from "@streamdown/code";
import type { BundledLanguage } from "streamdown";

import { createCodePlugin } from "@streamdown/code";

const streamdownCodePlugin = createCodePlugin();

function createPlainTokens(code: string): HighlightResult {
  return {
    bg: "transparent",
    fg: "inherit",
    tokens: code.split("\n").map((line) =>
      line === ""
        ? []
        : [
            {
              bgColor: "transparent",
              color: "inherit",
              content: line,
              htmlStyle: {},
              offset: 0,
            },
          ]
    ),
  };
}

export const sharedCodePlugin = {
  ...streamdownCodePlugin,
  highlight(options: HighlightOptions, callback?: (result: HighlightResult) => void) {
    if (
      !streamdownCodePlugin.supportsLanguage(options.language as BundledLanguage)
    ) {
      const fallback = createPlainTokens(options.code);
      callback?.(fallback);
      return fallback;
    }

    return streamdownCodePlugin.highlight(options, callback);
  },
};
