import { en, zh } from "./i18n-tables.js";

export type Locale = "zh-CN" | "en";

export { en, zh };

export function t(locale: Locale, key: string): string {
  const table = locale === "en" ? en : zh;
  return table[key] ?? key;
}
