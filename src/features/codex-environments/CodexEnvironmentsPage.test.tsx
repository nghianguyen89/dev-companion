import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CodexEnvironmentsPage } from "./CodexEnvironmentsPage";

it("shows a local Codex environment manager without credential-management controls", () => {
  const html = renderToStaticMarkup(<CodexEnvironmentsPage />);
  expect(html).toContain("Codex environments");
  expect(html).toContain("Credentials always remain with the official Codex CLI");
  expect(html).toContain("Add Codex environment");
  expect(html).not.toContain("Access token");
});
