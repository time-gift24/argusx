export type DevShowcase = {
  id: string;
  title: string;
  href: string;
  status: "Available" | "Experimental";
  summary: string;
  details: string;
  highlights: string[];
};

export const DEV_SHOWCASES: DevShowcase[] = [
  {
    id: "prompt-composer",
    title: "Prompt Composer",
    href: "/chat",
    status: "Available",
    summary:
      "Explore the docked prompt composer with agent and workflow targeting.",
    details:
      "Review the bottom-docked conversation surface that keeps agent and workflow choice visible without turning the chat entry into a tool panel.",
    highlights: [
      "Agent and workflow switching",
      "Grouped picker behavior",
      "Async submit and retry state",
    ],
  },
  {
    id: "stream",
    title: "Stream Playground",
    href: "/dev/stream",
    status: "Available",
    summary: "Inspect runtime stream surfaces and open and close behavior.",
    details:
      "Use this playground to validate runtime message sections, reasoning surfaces, and tool execution transitions without leaving the desktop shell.",
    highlights: ["Reasoning surface", "Tool state transitions"],
  },
  {
    id: "streamdown",
    title: "Streamdown Playground",
    href: "/dev/streamdown",
    status: "Available",
    summary: "Inspect markdown, code, math, and mermaid rendering.",
    details:
      "Use this page to verify streamed markdown fidelity, code treatments, equation rendering, and Mermaid support in the desktop runtime.",
    highlights: ["Markdown", "Code panels", "Mermaid"],
  },
] as const;
