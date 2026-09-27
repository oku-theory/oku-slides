# Polices livrées avec Oku Slides

Chargées par le rendu Typst au démarrage de l'app (et par `oku-slides --fonts assets/fonts`).

Suivies par Git LFS (`.gitattributes`) : installer Git LFS (`git lfs install`)
avant de cloner, ou `git lfs pull` dans un clone existant ; sinon ce dossier
ne contient que des pointeurs texte et le rendu n'a plus ses polices.

| Famille | Fichiers | Thème |
| --- | --- | --- |
| Newsreader | `newsreader-latin-standard-*.ttf` | papier (titres, contenu) |
| Instrument Sans | `instrument-sans-latin-standard-*.ttf` | papier (texte) |
| Inter | `inter-*.ttf` | pro |
| Noto Sans Arabic | `noto-sans-arabic-{400,700}.ttf` | tous (alphabet arabe) |
| Noto Sans Devanagari, Bengali, Thai | `noto-sans-{devanagari,bengali,thai}-{400,700}.ttf` | tous (hindi, bengali, thaï) |
| Noto Sans JP, SC, TC, KR | `noto-sans-{jp,sc,tc,kr}-{400,700}.otf` | tous (japonais, chinois, coréen) |

Versions statiques, sous-ensemble latin, converties depuis les paquets
Fontsource (`@fontsource-variable/newsreader`, `@fontsource-variable/instrument-sans`,
`@fontsource/inter`). Noto Sans Arabic : versions statiques « hinted » de
[notofonts/arabic](https://github.com/notofonts/arabic) (2022). Licence : SIL
Open Font License 1.1. Noto Sans Devanagari, Bengali et Thai : versions
statiques « hinted » de [notofonts](https://github.com/notofonts/notofonts.github.io)
(`fonts/<famille>/hinted/ttf`, Regular et Bold), OFL 1.1.

Noto Sans CJK : [notofonts/noto-cjk](https://github.com/notofonts/noto-cjk)
(`Sans/SubsetOTF`, version 2.004), réduites par `subset-cjk.py` au jeu national
de chaque région (JIS X 0208, GB 2312, Big5, KS X 1001 et tout le hangûl) :
≈ 23 Mo au lieu de 47. Les caractères rares hors de ces jeux restent sans glyphe.
Licence : SIL Open Font License 1.1. Pour les régénérer :
`uv run assets/fonts/subset-cjk.py <dossier des OTF téléchargés>`.

Les thèmes ne les nomment pas : `deck` ajoute ces polices (arabe, devanagari,
bengali, thaï, puis CJK) à chaque famille de `font`, la région de la langue du
deck en tête (`script-fallback` dans `typst/oku-slides/src/tokens.typ`).

À remplacer par les polices variables complètes (Google Fonts) pour couvrir
toutes les graisses et les autres alphabets.
