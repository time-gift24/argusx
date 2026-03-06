import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AnnotationPage } from "@/components/features/annotation/annotation-page";

const loadCatalogMock = vi.fn();

vi.mock("@/lib/stores/annotation-store", () => ({
  useAnnotationStore: (selector: (store: { loadCatalog: () => void }) => unknown) =>
    selector({
      loadCatalog: loadCatalogMock,
    }),
}));

vi.mock("@/components/features/annotation/annotation-workspace", () => ({
  AnnotationWorkspace: () => <div data-testid="annotation-workspace" />,
}));

describe("AnnotationPage", () => {
  it("renders breadcrumb hierarchy for the SOP route", () => {
    render(<AnnotationPage />);

    expect(screen.getByText("工作台")).toBeInTheDocument();
    expect(screen.getByText("SOP")).toBeInTheDocument();
    expect(screen.getByText("标注")).toBeInTheDocument();
    expect(screen.getByTestId("annotation-workspace")).toBeInTheDocument();
  });
});
