import type { Metadata } from "next";
import Script from "next/script";
import "./globals.css";
import { AppLayout } from "@/components/layouts";

export const metadata: Metadata = {
  title: "ArgusX",
  description: "ArgusX Desktop Application",
};

const themeInitScript = `
(() => {
  const storageKey = "argusx-theme";
  const root = document.documentElement;

  const resolveTheme = () => {
    try {
      const stored = window.localStorage.getItem(storageKey);
      if (stored === "light" || stored === "dark") {
        return stored;
      }
    } catch {}

    if (
      window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
    ) {
      return "dark";
    }

    return "light";
  };

  const theme = resolveTheme();
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
})();
`;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="antialiased">
        <Script id="argusx-theme-init" strategy="beforeInteractive">
          {themeInitScript}
        </Script>
        <AppLayout>{children}</AppLayout>
      </body>
    </html>
  );
}
