import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { InboxIcon } from "./icons";

const WEB = fileURLToPath(new URL("..", import.meta.url));

function sources(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "dist") {
      continue;
    }
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      sources(path, out);
    } else if (/\.(tsx?|css|html)$/.test(name)) {
      out.push(path);
    }
  }
  return out;
}

describe("tokens", () => {
  it("keep colour, type and spacing in CSS variables", () => {
    const css = readFileSync(join(WEB, "src/tokens.css"), "utf8");
    for (const group of ["--color-", "--font-", "--space-"]) {
      expect(css.match(new RegExp(`${group}[a-z0-9-]+:`, "g"))?.length).toBeGreaterThan(2);
    }
  });

  it("are the only place a colour literal appears", () => {
    const offenders = sources(join(WEB, "src"))
      .filter((path) => path.endsWith(".css") && !path.endsWith("tokens.css"))
      .filter((path) => /#[0-9a-f]{3,8}\b|rgba?\(/i.test(readFileSync(path, "utf8")));
    expect(offenders).toEqual([]);
  });
});

describe("icons", () => {
  it("draw inline as SVG with the current colour", () => {
    const html = renderToStaticMarkup(<InboxIcon />);
    expect(html).toMatch(/^<svg[^>]*aria-hidden="true"/);
    expect(html).toContain('stroke="currentColor"');
    expect(html).not.toMatch(/<(img|image|use)\b/);
  });
});

describe("web sources", () => {
  it("name no remote URL on any line", () => {
    // The generated contract carries only Rust doc comments; a URL there
    // would still be one line a reviewer should look at.
    const remote = /(https?:)?\/\/[a-z0-9.-]+\.[a-z]{2,}/i;
    const hits = sources(WEB).flatMap((path) =>
      readFileSync(path, "utf8")
        .split("\n")
        .map((line, index) => ({ line, at: `${relative(WEB, path)}:${index + 1}` }))
        .filter(({ line }) => remote.test(line))
        .map(({ at }) => at),
    );
    expect(hits).toEqual([]);
  });
});
