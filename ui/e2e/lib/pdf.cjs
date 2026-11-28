// A one-page text PDF built in memory, so no binary fixture is committed. The Rust test
// `pdf_hand_written_page_extracts_its_text` (crates/oku-slides-extract/tests/pdf.rs) writes the
// same bytes and checks that they extract to their text.

/** The text `text` as a PDF literal string: backslashes and parentheses escaped. */
const literal = (text) => `(${text.replace(/[\\()]/g, (c) => `\\${c}`)})`;

/**
 * A valid one-page PDF showing `text` (ASCII) in Helvetica with one `Tj`: a catalog, a page tree,
 * the page, the font and the content stream, then an xref table holding each object's offset.
 */
const textPdf = (text) => {
  const content = `BT /F1 24 Tf 72 720 Td ${literal(text)} Tj ET\n`;
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
    `<< /Length ${Buffer.byteLength(content, 'latin1')} >>\nstream\n${content}endstream`,
  ];
  let out = '%PDF-1.4\n';
  const offsets = objects.map((body, i) => {
    const at = Buffer.byteLength(out, 'latin1');
    out += `${i + 1} 0 obj\n${body}\nendobj\n`;
    return at;
  });
  const xref = Buffer.byteLength(out, 'latin1');
  // Each xref entry is exactly 20 bytes: 10-digit offset, 5-digit generation, type, space, LF.
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const at of offsets) out += `${String(at).padStart(10, '0')} 00000 n \n`;
  out += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(out, 'latin1');
};

module.exports = { textPdf };
