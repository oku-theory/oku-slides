import { describe, expect, test } from "bun:test";
import { resolveBundlePath, resolveLink } from "./knowledgeLinks";

const FROM = "/a/x.md";
const PATHS: ReadonlySet<string> = new Set(["/a/b.md", "/c.md", "/a/a b.md", "/a/ab.md", "/a/x.md"]);

describe("knowledge links", () => {
  test.each([
    ["https://x", { kind: "external", url: "https://x" }],
    ["mailto:a@b", { kind: "external", url: "mailto:a@b" }],
    ["HTTPS://x/y", { kind: "external", url: "https://x/y" }],
    ["JaVaScRiPt:alert(1)", { kind: "inert" }],
    [" javascript:alert(1)", { kind: "inert" }],
    ["java\tscript:alert(1)", { kind: "inert" }],
    ["&#106;avascript:alert(1)", { kind: "inert" }],
    ["&#x6A;avascript:alert(1)", { kind: "inert" }],
    ["javascript&colon;alert(1)", { kind: "inert" }],
    ["data:text/html,x", { kind: "inert" }],
    ["file:///etc/passwd", { kind: "inert" }],
    ["vbscript:x", { kind: "inert" }],
    ["//evil.com/x.md", { kind: "inert" }],
    ["\\\\host\\x", { kind: "inert" }],
    ["%2e%2e/x.md", { kind: "inert" }],
    ["a%2Fb.md", { kind: "inert" }],
    ["a%00b.md", { kind: "inert" }],
    ["a%5cb.md", { kind: "inert" }],
    ["a%zzb.md", { kind: "inert" }],
    ["/a/../../x", { kind: "inert" }],
    ["./b.md", { kind: "bundle", path: "/a/b.md" }],
    ["b.md", { kind: "bundle", path: "/a/b.md" }],
    ["../c.md", { kind: "bundle", path: "/c.md" }],
    ["/a/b.md#h", { kind: "bundle", path: "/a/b.md" }],
    ["b.md?x=1#h", { kind: "bundle", path: "/a/b.md" }],
    ["#h", { kind: "bundle", path: "/a/x.md" }],
    ["a%20b.md", { kind: "bundle", path: "/a/a b.md" }],
    ["./missing.md", { kind: "broken", path: "/a/missing.md" }],
    ["https://x/a b", { kind: "external", url: "https://x/a b" }],
    [" https://x\n/y ", { kind: "external", url: "https://x/y" }],
    ["ht tps://x", { kind: "external", url: "https://x" }],
    ["a b.md", { kind: "bundle", path: "/a/a b.md" }],
    ["&constructor;.md", { kind: "broken", path: "/a/&constructor;.md" }],
    ["&nbsp;https://x", { kind: "inert" }],
  ])("resolve link %p classified", (href, expected) => {
    expect(resolveLink(href, FROM, PATHS)).toEqual(expected as ReturnType<typeof resolveLink>);
  });

  test("resolve bundle path relative image resolved", () => {
    expect(resolveBundlePath("img/p.png", FROM)).toBe("/a/img/p.png");
  });

  test("resolve bundle path remote image null", () => {
    expect(resolveBundlePath("https://x/p.png", FROM)).toBeNull();
  });

  test("resolve bundle path escaping root null", () => {
    expect(resolveBundlePath("../../p.png", FROM)).toBeNull();
  });
});
