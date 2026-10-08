import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { InboxIcon } from "./InboxIcon";

describe("tokens and icons", () => {
  it("keeps color, type, and spacing in CSS variables", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const css = readFileSync(join(here, "tokens.css"), "utf8");
    expect(css).toContain("--color-ink");
    expect(css).toContain("--font-sans");
    expect(css).toContain("--space-4");
  });

  it("draws the inbox mark as an inline svg", () => {
    const html = renderToStaticMarkup(<InboxIcon />);
    expect(html.startsWith("<svg")).toBe(true);
  });
});
