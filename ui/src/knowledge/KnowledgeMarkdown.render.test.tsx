// Rendering of a knowledge file body: tables, raw HTML, sanitized links, bundle links, images,
// deep nesting and a render that throws.

import { afterEach, describe, expect, spyOn, test } from "bun:test";
import "../test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { KnowledgeMarkdown } = await import("./KnowledgeMarkdown");
const { t } = await import("../i18n");

const FROM = "/a/x.md";
const PATHS: ReadonlySet<string> = new Set(["/a/x.md", "/a/b.md", "/c.md"]);

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function render(
  text: string,
  { onOpen = () => {}, image = () => null }: { onOpen?: (path: string) => void; image?: (path: string) => string | null } = {},
): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(createElement(KnowledgeMarkdown, { text, from: FROM, paths: PATHS, onOpen, image })));
  return host;
}

describe("KnowledgeMarkdown", () => {
  test("knowledge markdown gfm table rendered", async () => {
    const host = await render("| a | b |\n|---|---|\n| 1 | 2 |");
    expect([...host.querySelectorAll("th")].map((c) => c.textContent)).toEqual(["a", "b"]);
    expect([...host.querySelectorAll("td")].map((c) => c.textContent)).toEqual(["1", "2"]);
  });

  test("knowledge markdown task list stays text", async () => {
    const host = await render("- [ ] todo\n- [x] done");
    expect(host.querySelector("input")).toBeNull();
    expect([...host.querySelectorAll("li")].map((c) => c.textContent)).toEqual(["[ ] todo", "[x] done"]);
  });

  test("knowledge markdown raw html literal text", async () => {
    const host = await render("<img src=x onerror=alert(1)>\n\nsome <b>bold</b> text");
    expect(host.querySelector("img")).toBeNull();
    expect(host.querySelector("b")).toBeNull();
    expect(host.textContent).toContain("<img src=x onerror=alert(1)>");
    expect(host.textContent).toContain("some <b>bold</b> text");
  });

  test("knowledge markdown javascript link plain text", async () => {
    const host = await render("[click](javascript:alert(1)) and [x](&#106;avascript:alert(1))");
    expect(host.querySelector("a")).toBeNull();
    expect(host.querySelector("button")).toBeNull();
    expect(host.textContent).toBe("click and x");
  });

  test("knowledge markdown angle bracket javascript link plain text", async () => {
    const host = await render("[click](<javascript:alert(1)>) <javascript:alert(2)>");
    expect(host.querySelector("a")).toBeNull();
    expect(host.querySelector("button")).toBeNull();
    expect(host.textContent).toBe("click javascript:alert(2)");
  });

  test("knowledge markdown reference link resolved like inline", async () => {
    const opened: string[] = [];
    const host = await render(
      "[web][w], [bad][j], [Doc][] and [doc]\n\n[w]: https://oku.dev\n[j]: javascript:alert(1)\n[doc]: ./b.md",
      { onOpen: (p) => opened.push(p) },
    );
    const a = host.querySelectorAll("a");
    expect(a.length).toBe(1);
    expect(a[0].getAttribute("href")).toBe("https://oku.dev");
    const buttons = host.querySelectorAll("button");
    expect(buttons.length).toBe(2);
    await act(async () => buttons[0].click());
    expect(opened).toEqual(["/a/b.md"]);
    // The definitions render nothing.
    expect(host.textContent).toBe("web, bad, Doc and doc");
  });

  test("knowledge markdown undefined reference stays literal", async () => {
    const host = await render("see [nothing] here");
    expect(host.querySelector("button")).toBeNull();
    expect(host.textContent).toBe("see [nothing] here");
  });

  test("knowledge markdown bare url stays text", async () => {
    const host = await render("go to https://oku.dev or www.oku.dev");
    expect(host.querySelector("a")).toBeNull();
    expect(host.textContent).toBe("go to https://oku.dev or www.oku.dev");
  });

  test("knowledge markdown external link opens in new window", async () => {
    const host = await render("[doc](https://oku.dev/a(b)) <https://oku.dev>");
    const a = [...host.querySelectorAll("a")];
    expect(a.map((e) => e.getAttribute("href"))).toEqual(["https://oku.dev/a%28b%29", "https://oku.dev"]);
    for (const e of a) {
      expect(e.getAttribute("target")).toBe("_blank");
      expect(e.getAttribute("rel")).toBe("noopener noreferrer");
    }
  });

  test("knowledge markdown external link href encoded title raw", async () => {
    const host = await render("[doc](<https://oku.dev/a b\\c>)");
    const a = host.querySelector("a");
    expect(a?.getAttribute("href")).toBe("https://oku.dev/a%20b%5Cc");
    expect(a?.getAttribute("title")).toBe("https://oku.dev/a b\\c");
  });

  test("knowledge markdown image alt plain text", async () => {
    const host = await render("![**a** _b_ `c` \\* [d](e)](/x.png)");
    expect(host.querySelector(".knowledge-image-alt")?.textContent).toBe("a b c * d");
  });

  test("knowledge markdown email autolink mailto", async () => {
    const host = await render("<a@b.com>");
    expect(host.querySelector("a")?.getAttribute("href")).toBe("mailto:a@b.com");
    expect(host.querySelector("a")?.textContent).toBe("a@b.com");
  });

  test("knowledge markdown deep blockquotes cut to text", async () => {
    const host = await render(`${">".repeat(10000)} deep`);
    // The walk stops at 64 levels: the rest is the source text.
    expect(host.querySelectorAll("blockquote").length).toBeLessThanOrEqual(64);
    expect(host.textContent).toContain(">>>");
    expect(host.textContent).toContain("deep");
  });

  test("knowledge markdown deep lists cut to text", async () => {
    const host = await render(Array.from({ length: 1000 }, (_, i) => `${"  ".repeat(i)}- item${i}`).join("\n"));
    expect(host.querySelectorAll("ul").length).toBeLessThanOrEqual(64);
    expect(host.textContent).toContain("item999");
  });

  test("knowledge markdown render error shows notice", async () => {
    const quiet = spyOn(console, "error").mockImplementation(() => {});
    try {
      const host = await render("see ![x](b.png) <b>", { image: () => { throw new Error("boom"); } });
      expect(host.querySelector(".md-failed")?.textContent).toBe(t("ui-markdown-failed"));
      expect(host.querySelector("pre.md-failed-text")?.textContent).toBe("see ![x](b.png) <b>");
      expect(host.querySelector("b")).toBeNull();
    } finally {
      quiet.mockRestore();
    }
  });

  test("knowledge markdown bundle link calls on open", async () => {
    const opened: string[] = [];
    const host = await render("[up](../c.md#top)", { onOpen: (p) => opened.push(p) });
    const button = host.querySelector("button");
    expect(button?.textContent).toBe("up");
    expect(host.querySelector("a")).toBeNull();
    await act(async () => button?.click());
    expect(opened).toEqual(["/c.md"]);
  });

  test("knowledge markdown missing bundle link broken", async () => {
    const host = await render("[gone](./gone.md)");
    const broken = host.querySelector("span.md-link-broken");
    expect(broken?.textContent).toBe("gone");
    expect(broken?.getAttribute("title")).toBe("ui-knowledge-link-broken");
    expect(host.querySelector("button")).toBeNull();
  });

  test("knowledge markdown remote image alt only", async () => {
    const asked: string[] = [];
    const host = await render("![a chart](https://evil.com/p.png)", { image: (p) => (asked.push(p), "blob:x") });
    expect(host.querySelector("img")).toBeNull();
    expect(host.querySelector("span.knowledge-image-alt")?.textContent).toBe("a chart");
    expect(asked).toEqual([]);
  });

  test("knowledge markdown asset image blob src", async () => {
    const host = await render("![chart](img/p.png) ![none](img/q.png)", {
      image: (p) => (p === "/a/img/p.png" ? "blob:http://x/1" : null),
    });
    const img = host.querySelectorAll("img");
    expect(img.length).toBe(1);
    expect(img[0].getAttribute("src")).toBe("blob:http://x/1");
    expect(img[0].getAttribute("alt")).toBe("chart");
    expect(host.querySelector("span.knowledge-image-alt")?.textContent).toBe("none");
  });

  test("knowledge markdown headings shifted and clamped at h6", async () => {
    const host = await render("# one\n\n## two\n\n##### five\n\n###### six\n\nset\n===");
    expect(host.querySelector("h1")).toBeNull();
    expect([...host.querySelectorAll("h2, h3, h4, h5, h6")].map((h) => `${h.tagName}:${h.textContent}`)).toEqual([
      "H2:one",
      "H3:two",
      "H6:five",
      "H6:six",
      "H2:set",
    ]);
  });
});
