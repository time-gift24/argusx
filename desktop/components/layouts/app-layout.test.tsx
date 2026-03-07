import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppLayout } from "@/components/layouts/app-layout";

let mockPathname = "/";

vi.mock("next/navigation", () => ({
  usePathname: () => mockPathname,
}));

vi.mock("@/hooks/use-mobile", () => ({
  useIsMobile: () => false,
}));

describe("AppLayout", () => {
  it("treats the root route as the chat workspace and hides the right module trigger", () => {
    mockPathname = "/";

    render(
      <AppLayout>
        <div>workspace</div>
      </AppLayout>
    );

    expect(
      screen.queryByRole("button", { name: /toggle chat panel/i })
    ).not.toBeInTheDocument();
    expect(screen.queryByText("工作台")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Provider 配置" })).toBeInTheDocument();
  });

  it("keeps the right module trigger on non-chat routes and shows breadcrumb in the header", () => {
    mockPathname = "/sop/annotation";

    const { container } = render(
      <AppLayout>
        <div>workspace</div>
      </AppLayout>
    );

    expect(
      screen.getByRole("button", { name: /toggle chat panel/i })
    ).toBeInTheDocument();
    expect(screen.getByText("ArgusX")).toBeInTheDocument();
    expect(screen.getByText("工作台")).toBeInTheDocument();
    expect(screen.getByText("SOP")).toBeInTheDocument();
    expect(screen.getByText("标注")).toBeInTheDocument();
    expect(screen.getByLabelText("打开 SOP 页面导航")).toBeInTheDocument();
    expect(
      container.querySelector('[data-slot="main-scroll-region"]')
    ).toHaveClass("min-h-0", "flex-1", "overflow-y-auto");
  });

  it("shows the Dev breadcrumb on the dev overview route", () => {
    mockPathname = "/dev";

    render(
      <AppLayout>
        <div>workspace</div>
      </AppLayout>
    );

    expect(screen.getByText("工作台")).toBeInTheDocument();
    expect(
      screen.getByRole("link", { current: "page", name: "Dev" })
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("打开 SOP 页面导航")
    ).not.toBeInTheDocument();
  });
});
