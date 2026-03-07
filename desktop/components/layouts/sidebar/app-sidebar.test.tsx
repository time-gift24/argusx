import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SidebarProvider } from "@/components/ui/sidebar";
import { AppSidebar } from "@/components/layouts/sidebar/app-sidebar";

let mockPathname = "/sop/annotation";

vi.mock("next/navigation", () => ({
  usePathname: () => mockPathname,
}));

vi.mock("@/hooks/use-mobile", () => ({
  useIsMobile: () => false,
}));

describe("AppSidebar", () => {
  it("marks the chat entry active on the root route", () => {
    mockPathname = "/";

    render(
      <SidebarProvider>
        <AppSidebar />
      </SidebarProvider>
    );

    expect(screen.getByRole("link", { name: /^对话$/i })).toHaveAttribute(
      "data-active",
      "true"
    );
  });

  it("keeps the nav flat, removes dashboard, and defaults to a lighter width", () => {
    mockPathname = "/sop/annotation";

    const { container } = render(
      <SidebarProvider>
        <AppSidebar />
      </SidebarProvider>
    );

    expect(
      screen.getByRole("link", { name: /^对话$/i })
    ).toHaveAttribute("href", "/chat");
    expect(screen.getByRole("link", { name: /sop 标注/i })).toHaveAttribute(
      "href",
      "/sop/annotation"
    );
    expect(
      screen.queryByRole("link", { name: /仪表板/i })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: /^sop$/i })
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /^标注$/i })).not.toBeInTheDocument();

    expect(
      container.querySelector('[data-slot="sidebar-wrapper"]')
    ).toHaveStyle("--sidebar-width: 208px");
  });

  it("renders a separate Dev group below the workspace group", () => {
    mockPathname = "/dev";

    const { container } = render(
      <SidebarProvider>
        <AppSidebar />
      </SidebarProvider>
    );

    const labels = Array.from(
      container.querySelectorAll('[data-slot="sidebar-group-label"]')
    ).map((element) => element.textContent);

    expect(labels).toEqual(["工作区", "Dev"]);
    expect(screen.getByRole("link", { name: /^dev$/i })).toHaveAttribute(
      "href",
      "/dev"
    );
  });

  it("keeps the Dev entry active across dev child routes", () => {
    mockPathname = "/dev/stream";

    render(
      <SidebarProvider>
        <AppSidebar />
      </SidebarProvider>
    );

    expect(screen.getByRole("link", { name: /^dev$/i })).toHaveAttribute(
      "data-active",
      "true"
    );
  });
});
