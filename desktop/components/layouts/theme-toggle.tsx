"use client";

import { Monitor, Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTheme } from "@/hooks";

export function ThemeToggle() {
  const { theme, toggleTheme, mounted } = useTheme();

  const ariaLabel = theme === "dark" ? "Switch to light" : "Switch to dark";

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="h-9 w-9"
      onClick={toggleTheme}
      aria-label={ariaLabel}
      title={ariaLabel}
    >
      {!mounted ? (
        <Monitor className="h-4 w-4" />
      ) : theme === "dark" ? (
        <Moon className="h-4 w-4" />
      ) : (
        <Sun className="h-4 w-4" />
      )}
    </Button>
  );
}
