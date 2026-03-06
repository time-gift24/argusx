import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AnnotationWorkspace } from "@/components/features/annotation/annotation-workspace";

describe("AnnotationWorkspace", () => {
  it("renders left and right regions with a wider desktop detail panel", () => {
    const { container } = render(<AnnotationWorkspace />);
    expect(screen.getByTestId("review-left-pane")).toBeInTheDocument();
    expect(screen.getByTestId("annotation-right-panel")).toBeInTheDocument();
    expect(container.firstChild).toHaveClass(
      "xl:grid-cols-[minmax(0,1fr)_420px]"
    );
  });
});
