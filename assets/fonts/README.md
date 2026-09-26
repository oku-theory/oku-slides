# Polices livrées avec Slider

Chargées par le rendu Typst au démarrage de l'app (et par `slider --fonts assets/fonts`).

| Famille | Fichiers | Thème |
| --- | --- | --- |
| Newsreader | `newsreader-latin-standard-*.ttf` | papier (titres, contenu) |
| Instrument Sans | `instrument-sans-latin-standard-*.ttf` | papier (texte) |
| Inter | `inter-*.ttf` | pro |
| Noto Sans Arabic | `noto-sans-arabic-{400,700}.ttf` | tous (alphabet arabe) |

Versions statiques, sous-ensemble latin, converties depuis les paquets
Fontsource (`@fontsource-variable/newsreader`, `@fontsource-variable/instrument-sans`,
`@fontsource/inter`). Noto Sans Arabic : versions statiques « hinted » de
[notofonts/arabic](https://github.com/notofonts/arabic) (2022). Licence : SIL
Open Font License 1.1.

À remplacer par les polices variables complètes (Google Fonts) pour couvrir
toutes les graisses et les autres alphabets.
