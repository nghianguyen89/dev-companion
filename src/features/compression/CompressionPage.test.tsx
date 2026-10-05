import { expect, it } from "vitest";
import { defaultCompressionExcludedPaths } from "./CompressionDefaults";

it("excludes common generated content by default without hiding source files", () => {
  expect(defaultCompressionExcludedPaths([
    { path: "node_modules", isDirectory: true, bytes: 0 },
    { path: "node_modules/react/index.js", isDirectory: false, bytes: 1 },
    { path: "src/app.ts", isDirectory: false, bytes: 1 },
    { path: "logs", isDirectory: true, bytes: 0 },
    { path: "logs/run.log", isDirectory: false, bytes: 1 },
    { path: "debug.log", isDirectory: false, bytes: 1 },
  ])).toEqual(["node_modules", "logs", "debug.log"]);
});
