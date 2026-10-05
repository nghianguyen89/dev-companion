import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { GuidePage } from "./GuidePage";

it("shows guidance for the current Codex migration workflow", () => {
  const html = renderToStaticMarkup(<GuidePage />);
  expect(html).toContain("Guide");
  expect(html).toContain("Move Codex accounts");
  expect(html).toContain(".chatgpt-projects");
  expect(html).toContain("File Transfer");
  expect(html).toContain("SourceTree");
  expect(html).toContain("RESTORE");
});
