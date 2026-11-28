// The knowledge files as a tree of directories, built from their paths, in the reader's order.

import type { KnowledgeNode } from "./types";

/** A directory of the knowledge bundle: its own page, its sub-directories, then its files. */
export interface TreeDir {
  /** Its last path segment; empty for the root. */
  name: string;
  /** Its path, ending in `/` (`/` for the root). */
  path: string;
  /** Its `index.md`, shown as the directory's own row. */
  index: KnowledgeNode | null;
  dirs: TreeDir[];
  files: KnowledgeNode[];
}

/** The last segment of `path`. */
function fileName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/** A path or a name as shown: bidi controls and other format characters removed. */
export function plainPath(path: string): string {
  return path.replace(/\p{Cf}/gu, "");
}

/** The file name of `path`, shown plain. */
export function pathLabel(path: string): string {
  return plainPath(fileName(path));
}

/** What a file sorts and shows by: its title, else its file name (shown plain). */
export function nodeLabel(node: KnowledgeNode): string {
  return node.title ?? pathLabel(node.path);
}

/** What a directory sorts by: its page's title, else its name. */
function dirLabel(dir: TreeDir): string {
  return dir.index?.title ?? dir.name;
}

/**
 * The tree of `files`: directories come from the paths, each `index.md` is its directory's page,
 * directories come before files, and both sort by title (else name) then path, in `locale`.
 */
export function buildTree(files: readonly KnowledgeNode[], locale: string): TreeDir {
  const root: TreeDir = { name: "", path: "/", index: null, dirs: [], files: [] };
  const dirs = new Map<string, TreeDir>([["/", root]]);
  const dirOf = (path: string): TreeDir => {
    const known = dirs.get(path);
    if (known) return known;
    // The session sends absolute paths; one without the leading `/` would never reach the root.
    if (!path.startsWith("/")) throw new Error(`knowledge path not absolute: ${path}`);
    const parentPath = path.slice(0, path.lastIndexOf("/", path.length - 2) + 1);
    const parent = dirOf(parentPath);
    const dir: TreeDir = { name: path.slice(parentPath.length, -1), path, index: null, dirs: [], files: [] };
    parent.dirs.push(dir);
    dirs.set(path, dir);
    return dir;
  };
  for (const file of files) {
    const dir = dirOf(file.path.slice(0, file.path.lastIndexOf("/") + 1));
    if (fileName(file.path) === "index.md") dir.index = file;
    else dir.files.push(file);
  }
  const collator = new Intl.Collator(locale);
  const order = (a: [string, string], b: [string, string]) => collator.compare(a[0], b[0]) || collator.compare(a[1], b[1]);
  const sort = (dir: TreeDir) => {
    dir.dirs.sort((a, b) => order([dirLabel(a), a.path], [dirLabel(b), b.path]));
    dir.files.sort((a, b) => order([nodeLabel(a), a.path], [nodeLabel(b), b.path]));
    dir.dirs.forEach(sort);
  };
  sort(root);
  return root;
}
