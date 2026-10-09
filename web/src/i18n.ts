// UI text by msgctxt key. i18n/mailune.pot is the only source; `npm run
// i18n` (scripts/i18n.py) writes the i18next catalog read here. Choosing a
// language and falling back to English is E4; until then the shell is English.

import english from "../../target/i18n/web/en.json";

type Catalog = { readonly [key: string]: string | Catalog };

/** The English text for `key`, such as `compose.send`. */
export function t(key: string): string {
  let node: string | Catalog | undefined = english as Catalog;
  for (const part of key.split(".")) {
    node = typeof node === "object" ? node[part] : undefined;
  }
  // A missing key shows itself rather than an empty control, so the gap is
  // visible in review instead of silently blank.
  return typeof node === "string" ? node : key;
}
