import { describe, expect, test } from "bun:test";
import { buildTree, type TreeDir } from "./knowledgeTree";
import type { KnowledgeNode } from "./types";

const node = (path: string, title: string | null = null, awaiting = false): KnowledgeNode => ({
  id: `id${path}`, path, title, kind: title === null ? null : "note", awaiting,
});
/** The tree as nested paths: each directory's index, sub-directories, then files. */
const shape = (dir: TreeDir): unknown => ({
  path: dir.path,
  index: dir.index?.path ?? null,
  dirs: dir.dirs.map(shape),
  files: dir.files.map((f) => f.path),
});

describe("buildTree", () => {
  test("buildTree index as dir page", () => {
    const root = buildTree([node("/index.md", "Home"), node("/a/index.md", "A"), node("/a/x.md", "X")], "en");
    expect(shape(root)).toEqual({
      path: "/", index: "/index.md", files: [],
      dirs: [{ path: "/a/", index: "/a/index.md", dirs: [], files: ["/a/x.md"] }],
    });
    expect(root.dirs[0].name).toBe("a");
  });

  test("buildTree sorted by title then path", () => {
    const root = buildTree([
      node("/z/b.md"), node("/c.md", "beta"), node("/b.md", "Alpha"), node("/a.md", "beta"), node("/é.md", "éclair"),
    ], "fr");
    // Directories first, then files by title (locale order: é sorts with e), then path.
    expect(root.dirs.map((d) => d.path)).toEqual(["/z/"]);
    expect(root.files.map((f) => f.path)).toEqual(["/b.md", "/a.md", "/c.md", "/é.md"]);
  });

  test("buildTree untitled uses name", () => {
    const root = buildTree([node("/b.md", "M"), node("/z/a.md"), node("/k.md"), node("/n.md", null, true)], "en");
    expect(root.files.map((f) => f.path)).toEqual(["/k.md", "/b.md", "/n.md"]);
  });

  test("buildTree deep paths nested", () => {
    const root = buildTree([node("/a/b/c/d.md", "D"), node("/a/e.md", "E"), node("/a/b/f.md", "F")], "en");
    expect(shape(root)).toEqual({
      path: "/", index: null, files: [],
      dirs: [{
        path: "/a/", index: null, files: ["/a/e.md"],
        dirs: [{
          path: "/a/b/", index: null, files: ["/a/b/f.md"],
          dirs: [{ path: "/a/b/c/", index: null, dirs: [], files: ["/a/b/c/d.md"] }],
        }],
      }],
    });
  });

  test("buildTree long raw paths keep their folders", () => {
    const dir = `/${"d".repeat(40)}/`;
    const deep = `${dir}${"e".repeat(30)}/`;
    const root = buildTree([node(`${dir}${"f".repeat(50)}.md`), node(`${deep}index.md`, "E")], "en");
    expect(shape(root)).toEqual({
      path: "/", index: null, files: [],
      dirs: [{
        path: dir, index: null, files: [`${dir}${"f".repeat(50)}.md`],
        dirs: [{ path: deep, index: `${deep}index.md`, dirs: [], files: [] }],
      }],
    });
  });

  test("buildTree relative path throws", () => {
    expect(() => buildTree([node("a/x.md")], "en")).toThrow("knowledge path not absolute");
  });
});
