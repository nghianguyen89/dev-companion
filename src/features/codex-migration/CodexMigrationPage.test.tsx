import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CodexMigrationPage } from "./CodexMigrationPage";

it("renders the multi-account migration choices and a guarded restore", () => {
  const html = renderToStaticMarkup(<CodexMigrationPage />);
  expect(html).toContain("Move Codex accounts");
  expect(html).toContain("Chat history and local databases");
  expect(html).toContain("Local ChatGPT project data");
  expect(html).toContain("Additional local Codex state");
  expect(html).toContain("Worktrees with uncommitted code");
  expect(html).toContain("Estimated backup input");
  expect(html).toContain("Migration backups");
  expect(html).not.toContain("Choose migration ZIP");
  expect(html).not.toContain("Restore on this machine");
});
