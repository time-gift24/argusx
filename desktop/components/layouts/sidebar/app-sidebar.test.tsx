import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SidebarProvider } from "@/components/ui/sidebar";
import { AppSidebar } from "@/components/layouts/sidebar/app-sidebar";

vi.mock("next/navigation", () => ({
  usePathname: () => "/sop/annotation",
}));

vi.mock("@/hooks/use-mobile", () => ({
  useIsMobile: () => false,
}));

describe("AppSidebar", () => {
  it("keeps the nav flat and points SOP entry to /sop/annotation", () => {
    render(
      <SidebarProvider>
        <AppSidebar />
      </SidebarProvider>
    );

    expect(screen.getByRole("link", { name: /sop 标注/i })).toHaveAttribute(
      "href",
      "/sop/annotation"
    );
    expect(
      screen.queryByRole("link", { name: /^sop$/i })
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /^标注$/i })).not.toBeInTheDocument();
  });
});
