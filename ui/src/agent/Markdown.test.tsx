import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Markdown } from "./Markdown";

const html = (text: string, agentWindow = false) =>
  renderToStaticMarkup(createElement(Markdown, { text, agentWindow }));

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

  test("Markdown http link opens elsewhere with its destination as title", () => {
    expect(html("[doc](https://oku.dev)")).toBe(
      '<p><a href="https://oku.dev" title="https://oku.dev" target="_blank" rel="noopener noreferrer">doc</a></p>',
    );
  });

  test("Markdown link href encoded backslash and space", () => {
    expect(html("[doc](<https://oku.dev/a b\\c>)")).toBe(
      '<p><a href="https://oku.dev/a%20b%5Cc" title="https://oku.dev/a b\\c" target="_blank" rel="noopener noreferrer">doc</a></p>',
    );
  });

  test("Markdown deep nesting renders as text past the cap", () => {
    const out = html(`${">".repeat(10000)} deep`);
    expect(out.split("<blockquote>").length - 1).toBeLessThanOrEqual(64);
    expect(out).toContain("deep");
  });

  test("Markdown deep nested lists render as text past the cap", () => {
    const out = html(Array.from({ length: 1000 }, (_, i) => `${"  ".repeat(i)}- item${i}`).join("\n"));
    expect(out.split("<ul>").length - 1).toBeLessThanOrEqual(64);
    expect(out).toContain("item999");
  });

  test("Markdown http link in the agent window shown broken", () => {
    expect(html("[doc](http://oku.dev) <http://oku.dev/x>", true)).toBe(
      '<p><span class="md-link-broken" title="ui-markdown-link-blocked">doc</span> ' +
        '<span><span class="md-link-broken" title="ui-markdown-link-blocked">http://oku.dev/x</span></span></p>',
    );
  });

  test("Markdown https link in the agent window opens elsewhere", () => {
    expect(html("[doc](https://oku.dev)", true)).toContain('<a href="https://oku.dev"');
  });

  test("Markdown http link in a workspace window opens elsewhere", () => {
    expect(html("[doc](http://oku.dev)")).toContain('<a href="http://oku.dev"');
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

  test("Markdown GFM table renders inside a focusable, named scrolling region", () => {
    expect(html("| a | b |\n|---|---|\n| 1 | 2 |")).toBe(
      '<div class="md-table" tabindex="0" role="region" aria-label="ui-agent-md-table"><table><thead><tr><th>a</th><th>b</th></tr></thead>'
        + "<tbody><tr><td>1</td><td>2</td></tr></tbody></table></div>",
    );
  });

  test("Markdown texte en cours de flux (gras non fermé) reste lisible", () => {
    expect(html("**Toto et")).toBe("<p>**Toto et</p>");
  });
});
