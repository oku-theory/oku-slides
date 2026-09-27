import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Markdown } from "./Markdown";

const html = (text: string) => renderToStaticMarkup(createElement(Markdown, { text }));

describe("Markdown des réponses de l'agent", () => {
  test("Markdown paragraphes séparés par une ligne vide donne deux paragraphes", () => {
    expect(html("un\n\ndeux")).toBe("<p>un</p><p>deux</p>");
  });

  test("Markdown gras et italique deviennent strong et em", () => {
    expect(html("**Toto** et *la Seine*")).toBe("<p><strong>Toto</strong> et <em>la Seine</em></p>");
  });

  test("Markdown liste à puces collée à un paragraphe devient une liste", () => {
    expect(html("deux cartes :\n- **A** : un\n- **B** : deux")).toBe(
      "<p>deux cartes :</p><ul><li><p><strong>A</strong> : un</p></li><li><p><strong>B</strong> : deux</p></li></ul>",
    );
  });

  test("Markdown liste numérotée garde son numéro de départ", () => {
    expect(html("3. trois\n4. quatre")).toBe('<ol start="3"><li><p>trois</p></li><li><p>quatre</p></li></ol>');
  });

  test("Markdown code en ligne et bloc de code", () => {
    expect(html("voir `slide`\n\n```json\n{\"a\": 1}\n```")).toBe(
      '<p>voir <code>slide</code></p><pre><code>{&quot;a&quot;: 1}</code></pre>',
    );
  });

  test("Markdown titre devient un titre discret", () => {
    expect(html("## Plan\ntexte")).toBe('<p class="md-heading">Plan</p><p>texte</p>');
  });

  test("Markdown lien http s'ouvre ailleurs", () => {
    expect(html("[doc](https://oku.dev)")).toBe('<p><a href="https://oku.dev" target="_blank" rel="noreferrer">doc</a></p>');
  });

  test("Markdown lien javascript reste du texte", () => {
    expect(html("[x](javascript:alert(1))")).toBe("<p><span>x</span></p>");
  });

  test("Markdown HTML brut est affiché comme du texte", () => {
    expect(html("<img src=x onerror=alert(1)>")).toBe("<p>&lt;img src=x onerror=alert(1)&gt;</p>");
  });

  test("Markdown échappement rend le caractère seul", () => {
    expect(html("\\*pas en italique\\*")).toBe("<p>*pas en italique*</p>");
  });

  test("Markdown citation et barré", () => {
    expect(html("> ~~vieux~~ neuf")).toBe("<blockquote><p><del>vieux</del> neuf</p></blockquote>");
  });

  test("Markdown tableau GFM devient un tableau", () => {
    expect(html("| a | b |\n|---|---|\n| 1 | 2 |")).toBe(
      "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table>",
    );
  });

  test("Markdown texte en cours de flux (gras non fermé) reste lisible", () => {
    expect(html("**Toto et")).toBe("<p>**Toto et</p>");
  });
});
