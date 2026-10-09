import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { type Catalog, ENGLISH, languages, loadLanguage, pickLanguage, t, translate } from "./i18n";

const CATALOGS = join(dirname(fileURLToPath(import.meta.url)), "../../target/i18n/web");

function catalog(code: string): Catalog {
  return JSON.parse(readFileSync(join(CATALOGS, `${code}.json`), "utf8")) as Catalog;
}

afterEach(async () => {
  await loadLanguage(ENGLISH);
});

describe("web catalogs", () => {
  it("are the ones scripts/i18n.py writes, English included", () => {
    expect(languages()).toContain(ENGLISH);
    expect(languages()).toContain("de");
  });

  it("load by language code", async () => {
    const english = catalog("en").app as Catalog;
    const german = catalog("de").app as Catalog;
    expect(german.compose).not.toBe(english.compose);

    expect(await loadLanguage("de")).toBe("de");
    expect(t("app.compose")).toBe(german.compose);
    expect(await loadLanguage("en")).toBe("en");
    expect(t("app.compose")).toBe(english.compose);
  });

  it("match a regional code to its language and anything unknown to English", async () => {
    expect(pickLanguage("de-AT")).toBe("de");
    expect(pickLanguage("DE_ch")).toBe("de");
    expect(pickLanguage("pt-BR", ["en", "pt", "pt-br"])).toBe("pt-br");
    expect(pickLanguage("pt-PT", ["en", "pt", "pt-br"])).toBe("pt");
    expect(await loadLanguage("xx-YY")).toBe(ENGLISH);
    expect(t("app.compose")).toBe((catalog("en").app as Catalog).compose);
  });

  it("fall back to English for a key the other catalog leaves out", () => {
    const english: Catalog = { thread: { re: "Re: {{0}}", summary: "Summary" } };
    const partial: Catalog = { thread: { summary: "Zusammenfassung" } };
    expect(translate([partial, english], "thread.summary")).toBe("Zusammenfassung");
    expect(translate([partial, english], "thread.re", ["Build"])).toBe("Re: Build");
    expect(translate([partial, english], "thread.nowhere")).toBe("thread.nowhere");
  });

  it("fill placeholders in the loaded language", async () => {
    await loadLanguage("de");
    const german = (catalog("de").thread as Catalog).re as string;
    expect(t("thread.re", "Build")).toBe(german.replace("{{0}}", "Build"));
  });
});
