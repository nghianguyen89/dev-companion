import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PetsPage } from "./PetsPage";

it("renders the validated local pet workflow", () => {
  const html = renderToStaticMarkup(<PetsPage />);
  expect(html).toContain("Install pet");
  expect(html).toContain("valid v2 pet.json");
});
