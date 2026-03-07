import { cjk } from "@streamdown/cjk";
import { createMathPlugin } from "@streamdown/math";
import { createMermaidPlugin } from "@streamdown/mermaid";

import { sharedCodePlugin } from "@/components/ai/shared-code-highlighter";

export const sharedMathPlugin = createMathPlugin({
  singleDollarTextMath: true,
});

export const sharedMermaidPlugin = createMermaidPlugin();

export const sharedStreamdownPlugins = {
  cjk,
  code: sharedCodePlugin,
  math: sharedMathPlugin,
  mermaid: sharedMermaidPlugin,
} as const;
