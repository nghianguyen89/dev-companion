import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AboutPage } from "./AboutPage";

it("shows the current version and release notes", () => {
  const html = renderToStaticMarkup(<AboutPage />);
  expect(html).toContain("About");
  expect(html).toContain("0.2.0");
  expect(html).toContain("Update history");
});
