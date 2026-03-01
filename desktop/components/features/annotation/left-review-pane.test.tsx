import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { AnnotationWorkspace } from "@/components/features/annotation/annotation-workspace";

describe("left pane trigger", () => {
  it("opens right panel when plain field is clicked", async () => {
    const user = userEvent.setup();
    render(<AnnotationWorkspace />);

    await user.click(screen.getByTestId("annotatable-field-case_title"));

    expect(screen.getAllByDisplayValue("case_title").length).toBeGreaterThan(0);
  });
});
