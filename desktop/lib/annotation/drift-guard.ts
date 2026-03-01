import Delta from "quill-delta";

export function reanchorByDelta(
  input: {
    start: number;
    end: number;
    selectedText?: string;
    currentTextAtRange?: string;
  },
  delta: Delta,
): { start: number; end: number; status?: "ok" | "orphaned" } {
  const start = delta.transformPosition(input.start);
  const end = delta.transformPosition(input.end);

  if (
    input.selectedText !== undefined &&
    input.currentTextAtRange !== undefined &&
    input.selectedText !== input.currentTextAtRange
  ) {
    return { start, end, status: "orphaned" };
  }

  return { start, end, status: "ok" };
}
