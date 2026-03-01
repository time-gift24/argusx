"use client";

import * as React from "react";

const THEME_STORAGE_KEY = "argusx-theme";
const DARK_MEDIA_QUERY = "(prefers-color-scheme: dark)";

export type ThemeMode = "light" | "dark";

type ThemeState = {
  theme: ThemeMode;
  hasStoredPreference: boolean;
};

function isThemeMode(value: string | null): value is ThemeMode {
  return value === "light" || value === "dark";
}

function readStoredTheme(): ThemeMode | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const raw = window.localStorage.getItem(THEME_STORAGE_KEY);
    return isThemeMode(raw) ? raw : null;
  } catch {
    return null;
  }
}

function getSystemTheme(): ThemeMode {
  if (typeof window === "undefined" || !window.matchMedia) {
    return "light";
  }

  return window.matchMedia(DARK_MEDIA_QUERY).matches ? "dark" : "light";
}

function applyTheme(theme: ThemeMode) {
  if (typeof document === "undefined") {
    return;
  }

  const root = document.documentElement;
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
}

function persistTheme(theme: ThemeMode) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // Ignore write failures (private mode or disabled storage).
  }
}

function getInitialThemeState(): ThemeState {
  if (typeof window === "undefined") {
    return { theme: "light", hasStoredPreference: false };
  }

  const storedTheme = readStoredTheme();
  if (storedTheme) {
    return { theme: storedTheme, hasStoredPreference: true };
  }

  return { theme: getSystemTheme(), hasStoredPreference: false };
}

export function useTheme() {
  const [mounted, setMounted] = React.useState(false);
  const [state, setState] = React.useState<ThemeState>(getInitialThemeState);

  React.useEffect(() => {
    setMounted(true);
  }, []);

  React.useEffect(() => {
    applyTheme(state.theme);
    if (state.hasStoredPreference) {
      persistTheme(state.theme);
    }
  }, [state.hasStoredPreference, state.theme]);

  React.useEffect(() => {
    if (state.hasStoredPreference || typeof window === "undefined" || !window.matchMedia) {
      return;
    }

    const mediaQuery = window.matchMedia(DARK_MEDIA_QUERY);
    const updateThemeFromSystem = (isDark: boolean) => {
      setState((current) =>
        current.hasStoredPreference
          ? current
          : { ...current, theme: isDark ? "dark" : "light" }
      );
    };

    const listener = (event: MediaQueryListEvent) => {
      updateThemeFromSystem(event.matches);
    };

    if (typeof mediaQuery.addEventListener === "function") {
      mediaQuery.addEventListener("change", listener);
      return () => mediaQuery.removeEventListener("change", listener);
    }

    // Fallback for older engines that only support addListener/removeListener.
    mediaQuery.addListener(listener);
    return () => mediaQuery.removeListener(listener);
  }, [state.hasStoredPreference]);

  const setTheme = React.useCallback((next: ThemeMode) => {
    setState({ theme: next, hasStoredPreference: true });
  }, []);

  const toggleTheme = React.useCallback(() => {
    setState((current) => ({
      theme: current.theme === "dark" ? "light" : "dark",
      hasStoredPreference: true,
    }));
  }, []);

  return {
    theme: state.theme,
    setTheme,
    toggleTheme,
    mounted,
  };
}
