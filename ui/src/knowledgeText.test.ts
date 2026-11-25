// Hidden characters of a knowledge file, revealed before the user reads and approves it.

import { expect, test } from "bun:test";
import { revealHidden } from "./knowledgeText";

test("revealHidden a tag character shows its code point", () => {
  expect(revealHidden("a\u{E0041}b")).toEqual({ text: "a⟨U+E0041⟩b", hidden: 1 });
});

test("revealHidden a zero-width space shows its code point", () => {
  expect(revealHidden("x\u200By")).toEqual({ text: "x⟨U+200B⟩y", hidden: 1 });
});

test("revealHidden a right-to-left override shows its code point", () => {
  expect(revealHidden("\u202Eevil")).toEqual({ text: "⟨U+202E⟩evil", hidden: 1 });
});

test("revealHidden line feeds and tabs stay as they are", () => {
  expect(revealHidden("one\n\ttwo")).toEqual({ text: "one\n\ttwo", hidden: 0 });
});

test("revealHidden other controls and private use show their code points", () => {
  expect(revealHidden("a\rb\u0007c\uE000")).toEqual({
    text: "a⟨U+000D⟩b⟨U+0007⟩c⟨U+E000⟩",
    hidden: 3,
  });
});

// The classes Rust's `bounds::is_label_invisible` drops, plus blank separators: each is revealed.
const REVEALED: [string, string][] = [
  ["a default-ignorable code point", "⁥"],
  ["a variation selector", "️"],
  ["a supplementary variation selector", "\u{E0100}"],
  ["a Mongolian free variation selector", "᠋"],
  ["the combining grapheme joiner", "͏"],
  ["a Hangul choseong filler", "ᅟ"],
  ["a Hangul jungseong filler", "ᅠ"],
  ["the Hangul filler", "ㅤ"],
  ["the half-width Hangul filler", "ﾠ"],
  ["the Braille blank", "⠀"],
  ["a Khmer inherent vowel", "឴"],
  ["the line separator", " "],
  ["the paragraph separator", " "],
  ["a no-break space", " "],
  ["an ideographic space", "　"],
  ["a musical symbol beam", "\u{1D173}"],
];

for (const [name, c] of REVEALED) {
  test(`revealHidden ${name} shows its code point`, () => {
    const code = (c.codePointAt(0) as number).toString(16).toUpperCase().padStart(4, "0");
    expect(revealHidden(`x${c}y`)).toEqual({ text: `x⟨U+${code}⟩y`, hidden: 1 });
  });
}

test("revealHidden a carriage return before a line feed stays, a lone one shows", () => {
  expect(revealHidden("a\r\nb\rc")).toEqual({ text: "a\r\nb⟨U+000D⟩c", hidden: 1 });
});

test("revealHidden CJK, accented and plain emoji text reveals nothing", () => {
  const text = "日本語の文章、中文句子。한국어 문장. Été à l'hôtel, naïve façade 🙂👍";
  expect(revealHidden(text)).toEqual({ text, hidden: 0 });
});

test("revealHidden an emoji ZWJ sequence shows its joiner", () => {
  expect(revealHidden("\u{1F468}‍\u{1F469}")).toEqual({
    text: "\u{1F468}⟨U+200D⟩\u{1F469}",
    hidden: 1,
  });
});
