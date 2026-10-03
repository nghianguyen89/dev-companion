import { expect, it } from "vitest";
import { resolveTheme } from "./theme";

it("honors an explicit theme and uses the system preference only in system mode", () => {
  expect(resolveTheme("light", true)).toBe("light");
  expect(resolveTheme("dark", false)).toBe("dark");
  expect(resolveTheme("system", false)).toBe("light");
  expect(resolveTheme("system", true)).toBe("dark");
});
