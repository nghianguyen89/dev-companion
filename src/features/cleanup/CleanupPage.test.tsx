import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CleanupStorage } from "./CleanupPage";

it("shows metadata-only storage groups without deletion controls and labels partial sizes", () => {
  const html = renderToStaticMarkup(<CleanupStorage usage={[
    { name: "plugins", bytes: 2 * 1024 * 1024, files: 4, skipped: 1, incomplete: false },
    { name: "", bytes: 1024, files: 1, skipped: 0, incomplete: true },
  ]} />);
  expect(html).toContain("plugins");
  expect(html).toContain("2.00 MiB");
  expect(html).toContain("including databases");
  expect(html).toContain("Partial inventory");
  expect(html).toContain("≥ ");
  expect(html).not.toContain("<button");
  expect(html).not.toContain("<input");
});
it("does not label a complete inventory as partial", () => {
  const html = renderToStaticMarkup(<CleanupStorage usage={[
    { name: "sessions", bytes: 1024, files: 1, skipped: 0, incomplete: false },
  ]} />);
  expect(html).not.toContain("Partial inventory");
  expect(html).not.toContain("≥ ");
});
