// Requêtes JSON vers `oku-slides serve` depuis la page (garde anti-CSRF : JSON déclaré).
const post = (p, endpoint, req) =>
  p.evaluate(
    ([e, r]) => fetch(e, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(r) }).then((x) => x.json()),
    [endpoint, req],
  );
module.exports = { post };
