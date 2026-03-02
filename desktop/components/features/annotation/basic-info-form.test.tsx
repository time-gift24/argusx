import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { BasicInfoForm } from "@/components/features/annotation/basic-info-form";
import { initialAnnotationState } from "@/lib/annotation/reducer";
import { useAnnotationStore } from "@/lib/stores/annotation-store";

describe("BasicInfoForm", () => {
  beforeEach(() => {
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
  });
});
