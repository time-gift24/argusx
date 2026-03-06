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
  });

  it("keeps the right module trigger on non-chat routes", () => {
    mockPathname = "/sop/annotation";

    render(
      <AppLayout>
        <div>workspace</div>
      </AppLayout>
    );

    expect(
      screen.getByRole("button", { name: /toggle chat panel/i })
    ).toBeInTheDocument();
  });
});
