import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Reader, readerSrcDoc } from "./Reader";

describe("reader", () => {
  it("renders the body in a sandboxed iframe with a strict CSP", () => {
    const html = renderToStaticMarkup(<Reader body="Can we meet Thursday afternoon?" />);
    expect(html).toContain("<iframe");
    expect(html).toContain('sandbox=""');
    expect(html).toContain("default-src");
    expect(html).toContain("img-src");
    const doc = readerSrcDoc('Hello <b>there</b>');
    expect(doc).toContain("default-src 'none'");
    expect(doc).toContain("img-src 'none'");
    expect(doc).toContain("&lt;b&gt;");
    expect(doc).not.toContain("<b>");
  });
});
