// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { READER_POLICY, Reader, readerDocument } from "./Reader";
import { Shell } from "./Shell";
import { fixtureBodies, fixtureThreads } from "./fixture";
import { t } from "./i18n";
import { fixtureBody } from "./message";
import { click, mount, pane } from "./render.test-utils";

let cleanup: (() => Promise<void>) | undefined;
afterEach(async () => {
  await cleanup?.();
  cleanup = undefined;
});

function frameDocument(): Document {
  return new DOMParser().parseFromString(readerDocument(fixtureBody), "text/html");
}

describe("reader frame", () => {
  it("withholds every sandbox permission and sends no referrer", async () => {
    const mounted = await mount(<Reader body={fixtureBody} />);
    cleanup = mounted.unmount;
    const frame = mounted.container.querySelector("iframe");
    expect(frame?.getAttribute("sandbox")).toBe("");
    expect(frame?.getAttribute("referrerpolicy")).toBe("no-referrer");
    expect(frame?.getAttribute("title")).toBe(t("thread.message_body"));
    expect(frame?.getAttribute("srcdoc")).toBe(readerDocument(fixtureBody));
    expect(frame?.hasAttribute("src")).toBe(false);
  });

  it("puts a policy that loads nothing ahead of the body", () => {
    const doc = frameDocument();
    const first = doc.head.firstElementChild;
    expect(first?.getAttribute("http-equiv")).toBe("Content-Security-Policy");
    expect(first?.getAttribute("content")).toBe(READER_POLICY);
    const directives = READER_POLICY.split(";").map((directive) => directive.trim());
    expect(directives).toContain("default-src 'none'");
    expect(READER_POLICY).not.toMatch(/unsafe|\*|https?:|data:|blob:/);
    expect(doc.querySelector("base")?.getAttribute("target")).toBe("_blank");
    expect(doc.querySelector("base")?.hasAttribute("href")).toBe(false);
  });

  it("shows the core's sanitised text with scripts and remote content gone", () => {
    const doc = frameDocument();
    expect(doc.body.textContent).toContain("Hello Ada,");
    expect(doc.body.textContent).toContain("Thanks,Grace");
    expect(doc.querySelectorAll("script, style, link, iframe, form, input, object, embed")).toHaveLength(0);
    for (const element of doc.body.querySelectorAll("*")) {
      for (const attribute of element.attributes) {
        expect(attribute.name.startsWith("on"), attribute.name).toBe(false);
        expect(attribute.value).not.toMatch(/^\s*javascript:/i);
        const loads = ["src", "srcset", "background", "poster", "data"].includes(attribute.name);
        if (loads) {
          expect(attribute.value, `${element.tagName} ${attribute.name}`).not.toMatch(/^\s*(https?:)?\/\//i);
        }
      }
    }
    expect(doc.querySelector("img")?.getAttribute("src")).toBe("cid:logo@mail");
  });

  it("is what the shell shows for a conversation with a body", async () => {
    const mounted = await mount(<Shell threads={fixtureThreads} bodies={fixtureBodies} />);
    cleanup = mounted.unmount;
    const list = pane(mounted.container, t("app.conversations"));
    const reading = pane(mounted.container, t("app.message"));
    await click([...list.querySelectorAll("button")].find((b) => b.textContent?.includes("Build notes")));
    expect(reading.querySelector("iframe")?.getAttribute("srcdoc")).toBe(readerDocument(fixtureBody));
  });

  it("does not accept a plain string as a body", () => {
    // @ts-expect-error A raw string has not been through the core's sanitiser.
    expect(readerDocument("<script>alert(1)</script>")).toContain("default-src 'none'");
  });
});
