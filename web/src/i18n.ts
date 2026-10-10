// UI text by msgctxt key. i18n/mailune.pot and the .po files are the only
// source; `npm run i18n` (scripts/i18n.py) writes one i18next catalog per
// language into target/i18n/web, and this module reads those files.
// English is bundled because every lookup falls back to it; the other
// catalogs load on demand, so a person downloads only their own language.

import english from "../../target/i18n/web/en.json";

export type Catalog = { readonly [key: string]: string | Catalog };

export const ENGLISH = "en";

const loaders = Object.fromEntries(
  Object.entries(import.meta.glob<Catalog>("../../target/i18n/web/*.json", { import: "default" })).map(
    ([path, load]) => [path.slice(path.lastIndexOf("/") + 1, -".json".length).toLowerCase(), load],
  ),
);

let active: Catalog = english;

/** The language codes a catalog exists for, such as `de` or `ja`. */
export function languages(): string[] {
  return Object.keys(loaders).sort();
}

/** The catalog for `code`: an exact match, then its primary subtag (`de-AT` → `de`), else English. */
export function pickLanguage(code: string, available: readonly string[] = languages()): string {
  const wanted = code.trim().toLowerCase().replace("_", "-");
  const primary = wanted.split("-")[0] ?? "";
  return [wanted, primary].find((candidate) => available.includes(candidate)) ?? ENGLISH;
}

/** Loads the catalog that best matches `code` and makes it the one `t` reads. Returns the code chosen. */
export async function loadLanguage(code: string): Promise<string> {
  const chosen = pickLanguage(code);
  const load = loaders[chosen];
  active = chosen === ENGLISH || !load ? english : await load();
  return chosen;
}

function lookup(catalog: Catalog, key: string): string | undefined {
  let node: string | Catalog | undefined = catalog;
  for (const part of key.split(".")) {
    node = typeof node === "object" ? node[part] : undefined;
  }
  return typeof node === "string" ? node : undefined;
}

/**
 * The text for `key` from the first catalog that has it, with `{0}`, `{1}`
 * filled from `args`. A catalog leaves out what is not translated yet.
 */
export function translate(catalogs: readonly Catalog[], key: string, args: readonly string[] = []): string {
  const text = catalogs.reduce<string | undefined>((found, catalog) => found ?? lookup(catalog, key), undefined);
  // A missing key shows itself rather than an empty control, so the gap is
  // visible in review instead of silently blank.
  if (text === undefined) {
    return key;
  }
  // scripts/i18n.py writes gettext's `{0}` as i18next's `{{0}}`.
  return text.replace(/\{\{(\d+)\}\}/g, (whole, index: string) => args[Number(index)] ?? whole);
}

/** The text for `key`, such as `compose.send`, in the loaded language, falling back to English. */
export function t(key: string, ...args: string[]): string {
  return translate([active, english], key, args);
}
