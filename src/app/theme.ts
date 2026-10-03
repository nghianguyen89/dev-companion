import type { AppConfiguration } from "../types/codex";

export function resolveTheme(theme: AppConfiguration["theme"] | undefined, systemPrefersDark: boolean) {
  return theme === "light" || theme === "dark" ? theme : systemPrefersDark ? "dark" : "light";
}
