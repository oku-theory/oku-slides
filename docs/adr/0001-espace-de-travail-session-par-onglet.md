# ADR-0001 : Espace de travail, une session par onglet, reprise v2 paresseuse

- **Statut :** accepté
- **Date :** 2026-09-26
- **Tâches :** t-142 (onglets et fenêtres), t-158 (restauration paresseuse), t-156 (cet ADR)
- **Spec :** [onglets-fenetres-design.md](../superpowers/specs/2026-09-26-onglets-fenetres-design.md)

## Contexte

Jusqu'à t-142, l'app n'ouvrait qu'une présentation à la fois : une `Session`
unique, reprise au lancement depuis `<data>/resume/` (format v1 :
`state.json`, `deck/`, `templates/`). L'utilisateur veut plusieurs
présentations ouvertes, en onglets répartis sur plusieurs fenêtres (plusieurs
écrans), dans une seule instance de l'app. Chaque onglet garde un état lourd
(rendu Typst, cache SVG, historique, mode modèles), et deux fenêtres doivent
pouvoir rendre en même temps. Tout doit revenir au relancement, sans que le
lancement coûte le rendu de chaque onglet.

## Décision

**Une `Session` par onglet, tenue par un `Workspace`.**

- `Workspace` (crate `oku-slides-session`) tient les fenêtres (`w-<n>`), leurs
  onglets ordonnés (`t-<n>`), l'onglet actif de chacune, la fenêtre active,
  et une session par onglet derrière un `Arc<Mutex<Slot>>`.
- `Slot` : `Ready(Session)`, `Dormant` (onglet repris, pas encore relu) ou
  `Closed` (fermé avant d'avoir été relu, plus jamais réveillé).
- **Ordre des verrous :** l'espace de travail, puis une session, jamais
  l'inverse. Une requête d'onglet ne verrouille que sa session. Aucune
  attente sur une session sous le verrou de l'espace de travail
  (`try_lock` seulement). Une session neuve se construit hors verrou.
  Un verrou empoisonné est repris.
- Identifiants jamais réutilisés (compteurs dans `workspace.json`), validés
  à la relecture (`t-`/`w-` et 1 à 18 chiffres) : ils deviennent des chemins
  et des labels de fenêtre Tauri.

**Reprise v2 :**

```
resume/
  workspace.json   { version: 2, next_tab, next_window, focused,
                     windows: [{ label, tabs, active }], kept, info: { t-<n>: TabInfo } }
  tabs/t-<n>/      la forme v1 : state.json, deck/, templates/
```

- Un écrivain par fichier : chaque session écrit `tabs/<id>/`, seul le
  `Workspace` écrit `workspace.json`.
- **Restauration paresseuse (t-158) :** aucune session n'est relue au
  lancement. La barre vient de `info` ; une session est relue à la
  première requête de son onglet. Les infos pouvant être en retard (arrêt
  brutal entre deux écritures), toute décision destructrice (fermer sans
  question, remplacer un onglet vierge) relit d'abord l'onglet ; relecture
  impossible = décision prudente.
- **Pas de migration v1 :** l'ancien `resume/state.json` est ignoré, jamais
  converti. On efface `<data>/resume/` à la main une fois.

**Hors périmètre :** un même deck dans deux fenêtres, onglets fermés
récupérables, migration de l'ancien format.

## Conséquences

**Positives**

- Deux fenêtres rendent en parallèle : chaque onglet a son verrou.
- `Session` change à peine : reprise, historique et mode modèles restent
  ceux d'un onglet unique, dans son propre dossier.
- Le lancement ne coûte plus le rendu de chaque onglet ; un onglet corrompu
  ne se découvre qu'à son usage, et ne coûte que lui (mis de côté, avis).
- Une entrée altérée de `workspace.json` ne coûte que son onglet.

**Négatives**

- Deux sources pour l'état d'un onglet endormi (`info` et son dossier) :
  il faut relire l'onglet avant toute décision qui détruit, et les tests
  doivent couvrir les infos en retard.
- L'ordre des verrous est une règle à tenir à la main, que le compilateur
  ne vérifie pas (documentée dans `ARCHITECTURE.md` du crate).
- `workspace.json` est réécrit à chaque changement de titre ou d'état d'un
  onglet (écriture sous le verrou de l'espace de travail).

**Neutres**

- Les utilisateurs de la v1 perdent une fois leur reprise (dossier ignoré,
  pas supprimé).
- Un onglet illisible découvert tard reste dans la barre, vierge, au lieu
  d'être retiré au lancement.

## Alternatives écartées

- **Une session unique qui change de deck à chaque onglet :** un seul
  rendu à la fois, historique et cache à sauvegarder et recharger à chaque
  changement d'onglet ; lent, et deux fenêtres se bloqueraient.
- **Un processus (ou une instance) par fenêtre :** pas d'instance unique,
  glisser un onglet entre fenêtres devient un transfert entre processus,
  et « ouvrir avec… » ne sait pas quelle instance viser.
- **Un seul `Mutex<Workspace>` englobant les sessions :** simple, mais tout
  rendu bloque toutes les fenêtres.
- **Migrer le format v1 :** code de compatibilité à garder pour un seul
  passage, contraire à la règle « pas de shim » du projet (MVP).
- **Tout relire au lancement (t-142 initial) :** simple et sans état en
  double, mais le lancement coûte le rendu de chaque onglet ouvert.
