import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { AnnotationWorkspace } from "@/components/features/annotation/annotation-workspace";
import { initialAnnotationState } from "@/lib/annotation/reducer";
import { useAnnotationStore } from "@/lib/stores/annotation-store";

describe("RightAnnotationPanel", () => {
  beforeEach(() => {
    useAnnotationStore.setState((current) => ({
      ...current,
      state: initialAnnotationState,
      catalog: [],
      catalogSource: null,
    }));
  });

  it("keeps controls disabled until an annotation target is selected", () => {
    render(<AnnotationWorkspace />);

    expect(screen.getByRole("combobox", { name: "违规检查项" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "提交标注" })).toBeDisabled();
  });

  it("reveals dynamic fields after rule selection", async () => {
    const user = userEvent.setup();
    render(<AnnotationWorkspace />);

    await user.click(screen.getByTestId("annotatable-field-case_title"));

    await user.click(screen.getByRole("combobox", { name: "违规检查项" }));
    await user.click(screen.getByRole("option", { name: "事实一致性" }));

    expect(screen.getByLabelText("问题说明")).toBeInTheDocument();
  });

  it("resets selected rule when switching to a new annotation target", async () => {
    const user = userEvent.setup();
    render(<AnnotationWorkspace />);

    await user.click(screen.getByTestId("annotatable-field-case_title"));
    await user.click(screen.getByRole("combobox", { name: "违规检查项" }));
    await user.click(screen.getByRole("option", { name: "事实一致性" }));
    expect(screen.getByLabelText("问题说明")).toBeInTheDocument();

    await user.click(screen.getByTestId("annotatable-field-case_summary"));

    expect(screen.getByRole("combobox", { name: "违规检查项" })).toHaveTextContent("请选择");
    expect(screen.queryByLabelText("问题说明")).not.toBeInTheDocument();
  });

  it("shows quill location metadata for rich-text selection", () => {
    useAnnotationStore.setState((current) => ({
      ...current,
      state: {
        items: [
          {
            id: "ann-rich-1",
            location: {
              source_type: "rich_text_selection",
              panel: "paragraph_detail",
              section_id: "paragraph-1",
              field_key: "paragraph.summary",
              node_id: "paragraph.summary-node",
              start_offset: 2,
              end_offset: 5,
              selected_text: "CDE",
            },
            ruleCode: null,
            payload: {},
            status: "draft",
            updatedAt: 1,
          },
        ],
        activeId: "ann-rich-1",
      },
    }));

    render(<AnnotationWorkspace />);

    expect(screen.getByDisplayValue("paragraph.summary")).toBeInTheDocument();
    expect(screen.getByDisplayValue("paragraph.summary-node")).toBeInTheDocument();
    expect(screen.getByDisplayValue("2")).toBeInTheDocument();
    expect(screen.getByDisplayValue("5")).toBeInTheDocument();
    expect(screen.getByDisplayValue("CDE")).toBeInTheDocument();
  });
});
