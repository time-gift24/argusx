import { readFileSync } from "node:fs";
import path from "node:path";

describe("desktop/app/globals.css", () => {
  it("includes streamdown dist source for Tailwind class extraction", () => {
    const globalsCssPath = path.resolve(__dirname, "globals.css");
    const css = readFileSync(globalsCssPath, "utf8");

    expect(css).toContain('@source "../node_modules/streamdown/dist/*.js";');
  });
});
