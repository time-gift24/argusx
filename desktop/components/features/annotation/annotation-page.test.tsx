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
  it("renders only the local page title block and workspace", () => {
    render(<AnnotationPage />);

    expect(screen.queryByText("工作台")).not.toBeInTheDocument();
    expect(screen.queryByText(/^SOP$/)).not.toBeInTheDocument();
    expect(screen.queryByLabelText("打开 SOP 页面导航")).not.toBeInTheDocument();
    expect(screen.getByText("SOP 标注工作台")).toBeInTheDocument();
    expect(screen.getByTestId("annotation-workspace")).toBeInTheDocument();
  });
});
