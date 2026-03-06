import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

type TauriConfig = {
  app: {
    windows: Array<{
      width: number;
      height: number;
      minWidth?: number;
      minHeight?: number;
      center?: boolean;
    }>;
  };
};

describe("tauri.conf.json", () => {
  it("uses large-screen desktop window defaults", () => {
    const configPath = path.join(
      path.dirname(fileURLToPath(import.meta.url)),
      "tauri.conf.json"
    );
    const config = JSON.parse(
      readFileSync(configPath, "utf8")
    ) as TauriConfig;

    expect(config.app.windows).toHaveLength(1);
    expect(config.app.windows[0]).toMatchObject({
      width: 1728,
      height: 1080,
      minWidth: 1440,
      minHeight: 900,
      center: true,
    });
  });
});
