// Where a link or an image of a knowledge file points: the web, another bundle file, or nowhere.
//
// The raw destination is classified once, and the value classified is the value emitted. Only
// `http`, `https` and `mailto` leave the bundle; every other scheme, a protocol-relative or UNC
// target, an encoded dot or slash, a NUL, a backslash or a climb above the root is inert.

/** What a link of a knowledge file does. */
export type KnowledgeLink =
  | { kind: "external"; url: string }
  | { kind: "bundle"; path: string }
  | { kind: "broken"; path: string }
  | { kind: "inert" };

const EXTERNAL = new Set(["http", "https", "mailto"]);

/** The named character references a destination is decoded with; any other stays literal. */
const NAMED: Record<string, string> = {
  amp: "&", lt: "<", gt: ">", quot: "\"", apos: "'", colon: ":", sol: "/", bsol: "\\",
  period: ".", num: "#", quest: "?", percnt: "%", Tab: "\t", NewLine: "\n", nbsp: "\u00a0",
};

/** A backslash escape of ASCII punctuation, or a character reference, as CommonMark reads them. */
const ESCAPE_OR_REFERENCE = /\\([!-/:-@[-`{-~])|&(?:#(\d{1,7})|#[xX]([0-9a-fA-F]{1,6})|([A-Za-z][A-Za-z0-9]*));/g;

/** ASCII controls and spaces: dropped to find a scheme, so ` java\tscript:` reads as `javascript:`. */
const BLANK = /[\u0000-\u0020\u007f]/g;

/** A code point as CommonMark decodes a numeric reference: invalid ones become U+FFFD. */
function codePoint(n: number): string {
  return n === 0 || n > 0x10ffff || (n >= 0xd800 && n <= 0xdfff) ? "\ufffd" : String.fromCodePoint(n);
}

/**
 * The destination as CommonMark means it: lezer hands the source text, with its escapes and
 * character references undecoded. Controls and spaces at the ends are trimmed and line breaks
 * and tabs inside dropped; an inner space stays (`<a b.md>` names `a b.md`, never `ab.md`).
 */
function clean(raw: string): string {
  const decoded = raw.replace(ESCAPE_OR_REFERENCE, (all, punct?: string, dec?: string, hex?: string, name?: string) => {
    if (punct !== undefined) return punct;
    if (dec !== undefined) return codePoint(Number.parseInt(dec, 10));
    if (hex !== undefined) return codePoint(Number.parseInt(hex, 16));
    return name !== undefined && Object.hasOwn(NAMED, name) ? NAMED[name] : all;
  });
  return decoded.replace(/^[\u0000-\u0020\u007f]+|[\u0000-\u0020\u007f]+$/g, "").replace(/[\t\n\r]/g, "");
}

/** The lowercased scheme: a `:` before the first `/`, `?` or `#`; `null` without one. */
function schemeOf(target: string): string | null {
  const colon = target.indexOf(":");
  if (colon < 0) return null;
  const stop = target.search(/[/?#]/);
  return stop < 0 || colon < stop ? target.slice(0, colon).toLowerCase() : null;
}

/** A cleaned, scheme-less target resolved against `from`'s directory; `null` when inert. */
function bundlePath(target: string, from: string): string | null {
  if (target.startsWith("//") || target.startsWith("\\")) return null;
  const end = target.search(/[?#]/);
  const raw = end < 0 ? target : target.slice(0, end);
  // An encoded dot or slash would hide a `..` or a separator from the checks below.
  if (/%2e|%2f/i.test(raw)) return null;
  let decoded: string;
  try {
    decoded = decodeURIComponent(raw);
  } catch {
    // A malformed escape names no file.
    return null;
  }
  if (/[\u0000-\u001f\u007f\\]/.test(decoded)) return null;
  // A bare `#part` or `?query` points at the file itself.
  if (decoded === "") return from;
  const parts = decoded.startsWith("/") ? [] : from.split("/").slice(1, -1);
  for (const segment of decoded.split("/")) {
    if (segment === "" || segment === ".") continue;
    if (segment === "..") {
      if (parts.length === 0) return null;
      parts.pop();
    } else parts.push(segment);
  }
  return `/${parts.join("/")}`;
}

/**
 * The bundle path a link or image `target` of the file at `from` names, or `null` when it names
 * none (a scheme, a remote or an inert target). Shared by links and images.
 */
export function resolveBundlePath(target: string, from: string): string | null {
  const cleaned = clean(target);
  return schemeOf(cleaned.replace(BLANK, "")) === null ? bundlePath(cleaned, from) : null;
}

/** What the link `href` of the file at `from` does, `paths` being the bundle's files. */
export function resolveLink(href: string, from: string, paths: ReadonlySet<string>): KnowledgeLink {
  const cleaned = clean(href);
  const scheme = schemeOf(cleaned.replace(BLANK, ""));
  if (scheme !== null) {
    // The emitted scheme is the one classified; the rest keeps its inner spaces (encoded later).
    return EXTERNAL.has(scheme) ? { kind: "external", url: scheme + cleaned.slice(cleaned.indexOf(":")) } : { kind: "inert" };
  }
  const path = bundlePath(cleaned, from);
  if (path === null) return { kind: "inert" };
  return paths.has(path) ? { kind: "bundle", path } : { kind: "broken", path };
}
