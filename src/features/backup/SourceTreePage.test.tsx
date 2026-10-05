import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SourceTreePage } from "./SourceTreePage";

it("keeps SourceTree recovery inside the configuration bundle list", () => {
  const html = renderToStaticMarkup(<SourceTreePage />);
  expect(html).toContain("SourceTree");
  expect(html).toContain("configuration");
  expect(html).toContain("Create encrypted bundle");
  expect(html).toContain("New bundle password");
  expect(html).toContain("Use at least 6 characters.");
  expect(html).toContain("Created configuration bundles");
  expect(html).not.toContain("Choose configuration bundle");
  expect(html).toContain("Windows");
});
