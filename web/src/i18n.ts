// UI text by msgctxt key. i18n/mailune.pot is the only source; `npm run
// i18n` (scripts/i18n.py) writes the i18next catalog read here. Choosing a
// language and falling back to English is E4; until then the shell is English.

import english from "../../target/i18n/web/en.json";

type Catalog = { readonly [key: string]: string | Catalog };

/** The English text for `key`, such as `compose.send`, with `{0}`, `{1}` filled from `args`. */
export function t(key: string, ...args: string[]): string {
  let node: string | Catalog | undefined = english as Catalog;
  for (const part of key.split(".")) {
    node = typeof node === "object" ? node[part] : undefined;
  }
  // A missing key shows itself rather than an empty control, so the gap is
  // visible in review instead of silently blank.
  if (typeof node !== "string") {
    return key;
  }
  // scripts/i18n.py writes gettext's `{0}` as i18next's `{{0}}`.
  return node.replace(/\{\{(\d+)\}\}/g, (whole, index: string) => args[Number(index)] ?? whole);
}
