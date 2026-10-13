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
| Noto Serif JP, SC, TC, KR | `noto-serif-{jp,sc,tc,kr}-{400,700}.otf` | papier (titres en japonais, chinois, coréen) |
| Carlito | `carlito-{400,700}{,-italic}.ttf` | import PPTX (substitut de Calibri) |
| Caladea | `caladea-{400,700}{,-italic}.ttf` | import PPTX (substitut de Cambria) |
| Liberation Sans, Serif, Mono | `liberation-{sans,serif,mono}-{400,700}{,-italic}.ttf` | import PPTX (substituts d'Arial et Helvetica, Times New Roman, Courier New) |

Versions statiques, sous-ensemble latin, converties depuis les paquets
Fontsource (`@fontsource-variable/newsreader`, `@fontsource-variable/instrument-sans`,
`@fontsource/inter`). Noto Sans Arabic : versions statiques « hinted » de
[notofonts/arabic](https://github.com/notofonts/arabic) (2022). Licence : SIL
Open Font License 1.1. Noto Sans Devanagari, Bengali et Thai : versions
statiques « hinted » de [notofonts](https://github.com/notofonts/notofonts.github.io)
(`fonts/<famille>/hinted/ttf`, Regular et Bold), OFL 1.1.

Noto Sans et Serif CJK : [notofonts/noto-cjk](https://github.com/notofonts/noto-cjk)
(`Sans/SubsetOTF` et `Serif/SubsetOTF`), réduites par `subset-cjk.py` au jeu
national de chaque région (JIS X 0208, GB 2312, Big5, KS X 1001 et tout le
hangûl) : Sans ≈ 23 Mo au lieu de 47, Serif ≈ 34 Mo au lieu de 68. Les
caractères rares hors de ces jeux restent sans glyphe. Licence : SIL Open Font
License 1.1. Pour les régénérer :
`uv run assets/fonts/subset-cjk.py <dossier des OTF téléchargés> [Sans|Serif]`.

Les thèmes ne les nomment pas : `deck` ajoute ces polices (arabe, devanagari,
bengali, thaï, puis CJK) à chaque famille de `font`, la région de la langue du
deck en tête (`script-fallback` dans `typst/oku-slides/src/tokens.typ`). Le
jeton `script-style` choisit, par rôle, le CJK sans ou serif (papier : titres
en serif).

À remplacer par les polices variables complètes (Google Fonts) pour couvrir
toutes les graisses et les autres alphabets.

Substituts de l'import PPTX (`oku-slides-import`, `fonts.rs`) : polices aux
métriques compatibles, prises quand la police d'une présentation n'est ni
embarquée ni installée, pour que le texte garde ses retours à la ligne.
Carlito : [google/fonts](https://github.com/google/fonts) `ofl/carlito`
(projet [googlefonts/carlito](https://github.com/googlefonts/carlito)),
Caladea : [google/fonts](https://github.com/google/fonts) `ofl/caladea`
(projet [huertatipografica/Caladea](https://github.com/huertatipografica/Caladea)),
Liberation 2.1.5 : [liberationfonts](https://github.com/liberationfonts/liberation-fonts)
(`liberation-fonts-ttf-2.1.5`). Licence : SIL Open Font License 1.1, texte
joint (`carlito-OFL.txt`, `caladea-OFL.txt`, `liberation-LICENSE.txt`). Livrées
avec l'app, elles ne sont pas recopiées dans les decks importés.
