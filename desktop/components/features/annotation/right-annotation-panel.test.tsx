import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { RightAnnotationPanel } from "@/components/features/annotation/right-annotation-panel";

describe("RightAnnotationPanel", () => {
  it("reveals dynamic fields after rule selection", async () => {
    const user = userEvent.setup();
    render(<RightAnnotationPanel />);

    await user.click(screen.getByRole("combobox", { name: "违规检查项" }));
    await user.click(screen.getByRole("option", { name: "事实一致性" }));

    expect(screen.getByLabelText("问题说明")).toBeInTheDocument();
  });
});
