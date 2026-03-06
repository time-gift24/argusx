import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ThemeToggle } from "@/components/layouts/theme-toggle";

const useThemeMock = vi.fn();

vi.mock("@/hooks", () => ({
  useTheme: () => useThemeMock(),
}));

describe("ThemeToggle", () => {
  it("uses a stable label before mount to avoid hydration mismatch", () => {
    useThemeMock.mockReturnValue({
      theme: "dark",
      toggleTheme: vi.fn(),
      mounted: false,
    });

    render(<ThemeToggle />);

    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-label", "切换到深色");
    expect(button).toHaveAttribute("title", "切换到深色");
  });

  it("uses actual theme label after mount", () => {
    useThemeMock.mockReturnValue({
      theme: "dark",
      toggleTheme: vi.fn(),
      mounted: true,
    });

    render(<ThemeToggle />);

    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-label", "切换到浅色");
    expect(button).toHaveAttribute("title", "切换到浅色");
  });
});
