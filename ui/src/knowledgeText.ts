// A received knowledge file's text made safe to read: every hidden character shows its code point.

/**
 * Characters that draw nothing, or a blank that is not a plain space.
 *
 * Format, private-use, unassigned and default-ignorable code points (variation selectors,
 * joiners, Hangul fillers, Khmer inherent vowels included), what Rust's
 * `bounds::is_label_invisible` adds (the Braille blank, musical symbol beams), line and
 * paragraph separators, every space separator but U+0020, every control but line feed and tab,
 * and a carriage return not followed by a line feed (`\r\n` is a newline).
 */
const HIDDEN = new RegExp(
  [
    "[\\p{Cf}\\p{Co}\\p{Cn}\\p{Default_Ignorable_Code_Point}\\p{Zl}\\p{Zp}",
    "\\u2800\\u{1D159}-\\u{1D15A}\\u{1D173}-\\u{1D17A}]",
    "|[^\\P{Zs} ]|[^\\P{Cc}\\n\\t\\r]|\\r(?!\\n)",
  ].join(""),
  "gu",
);

/**
 * `text` with each hidden character written `⟨U+XXXX⟩`, and how many there were.
 *
 * The user approves what the agent will read: a tag character, a zero-width space or a bidi
 * override must not hide instructions from them.
 */
export function revealHidden(text: string): { text: string; hidden: number } {
  let hidden = 0;
  const shown = text.replace(HIDDEN, (c) => {
    hidden += 1;
    const point = c.codePointAt(0);
    if (point === undefined) throw new Error("hidden character match is empty");
    const code = point.toString(16).toUpperCase().padStart(4, "0");
    return `⟨U+${code}⟩`;
  });
  return { text: shown, hidden };
}
