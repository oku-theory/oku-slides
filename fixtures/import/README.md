# Fixtures d'import (t-692)

- `minimal/`, `kitchen/` : OOXML décompressé écrit à la main, zippé à la
  volée par `crates/oku-slides-import/tests/common`.
- `libreoffice.pptx` : `kitchen` réécrit par LibreOffice (aller-retour).
- `alto-wealth.pptx` : deck d'exemple fourni par Matthieu le 2026-10-09,
  généré par PptxGenJS, versionné avec son accord pour les tests.
  `alto-wealth.pdf` (LFS) : son rendu de référence, produit séparément
  (« appifact kit ») ; cible visuelle des lots 4 et 6, lu par aucun test du lot 1.
