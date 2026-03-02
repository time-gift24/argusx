import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { BasicInfoForm } from "@/components/features/annotation/basic-info-form";
import { initialAnnotationState } from "@/lib/annotation/reducer";
import { useAnnotationStore } from "@/lib/stores/annotation-store";

const { toastInfoMock } = vi.hoisted(() => ({
  toastInfoMock: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    info: toastInfoMock,
  },
}));

describe("BasicInfoForm", () => {
  beforeEach(() => {
    toastInfoMock.mockReset();
    useAnnotationStore.setState((current) => ({
      ...current,
      state: initialAnnotationState,
      catalog: [],
      catalogSource: null,
    }));
  });

  it("keeps existing base fields and shows sop_id/name", () => {
    render(<BasicInfoForm />);

    expect(screen.getByText("案件标题")).toBeInTheDocument();
    expect(screen.getByText("案件摘要")).toBeInTheDocument();
    expect(screen.getByText("SOP ID")).toBeInTheDocument();
    expect(screen.getByText("SOP 名称")).toBeInTheDocument();
  });

  it("opens annotation for sop fields as plain_field location", async () => {
    const user = userEvent.setup();
    render(<BasicInfoForm />);

    await user.click(screen.getByTestId("annotatable-field-sop_id"));

    const state = useAnnotationStore.getState().state;
    const active = state.items.find((item) => item.id === state.activeId);

    expect(active?.location.source_type).toBe("plain_field");
    expect(active?.location.field_key).toBe("sop_id");
    expect(active?.location.selected_text).toBe("sop-001");
    expect(toastInfoMock).toHaveBeenCalledWith("新的标注事件", {
      description: "SOP ID",
    });
  });

  it("does not emit toast for existing plain-field annotation", async () => {
    useAnnotationStore.setState((current) => ({
      ...current,
      state: {
        items: [
          {
            id: "ann-base-1",
            location: {
              source_type: "plain_field",
              panel: "basic_info",
              section_id: "base",
              field_key: "sop_id",
              node_id: "sop_id",
              start_offset: null,
              end_offset: null,
              selected_text: "sop-001",
            },
            ruleCode: null,
            payload: {},
            status: "draft",
            updatedAt: 1,
          },
        ],
        activeId: "ann-base-1",
      },
    }));

    const user = userEvent.setup();
    render(<BasicInfoForm />);
    await user.click(screen.getByTestId("annotatable-field-sop_id"));

    expect(toastInfoMock).not.toHaveBeenCalled();
  });

  it("uses subtle active highlight and keeps non-active draft unhighlighted", () => {
    useAnnotationStore.setState((current) => ({
      ...current,
      state: {
        items: [
          {
            id: "ann-active",
            location: {
              source_type: "plain_field",
              panel: "basic_info",
              section_id: "base",
              field_key: "sop_id",
              node_id: "sop_id",
              start_offset: null,
              end_offset: null,
              selected_text: "sop-001",
            },
            ruleCode: null,
            payload: {},
            status: "draft",
            updatedAt: 1,
          },
          {
            id: "ann-draft-non-active",
            location: {
              source_type: "plain_field",
              panel: "basic_info",
              section_id: "base",
              field_key: "case_title",
              node_id: "case_title",
              start_offset: null,
              end_offset: null,
              selected_text: "某行政处罚案件",
            },
            ruleCode: null,
            payload: {},
            status: "draft",
            updatedAt: 1,
          },
        ],
        activeId: "ann-active",
      },
    }));

    render(<BasicInfoForm />);

    const activeField = screen.getByTestId("annotatable-field-sop_id");
    const nonActiveDraftField = screen.getByTestId("annotatable-field-case_title");

    expect(activeField).toHaveClass("bg-muted");
    expect(activeField).not.toHaveClass("border-emerald-500");
    expect(nonActiveDraftField).toHaveClass("bg-background");
  });
});
