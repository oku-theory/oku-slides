# ADR-0003 : Règle d'origine des serveurs HTTP locaux

- **Statut :** accepté
- **Date :** 2026-10-09
- **Tâches :** t-89 (première garde), t-419 (passage à axum), t-189 (cette
  décision), t-193 (avertissement hors boucle locale)
- **Code :** `crates/oku-slides-cli/src/guard.rs`

## Contexte

`oku-slides serve` ouvre un serveur HTTP sur la machine (par défaut
`127.0.0.1`) pour l'interface du navigateur : lecture et édition du deck,
flux d'évènements (SSE), agent. Écouter sur la boucle locale ne suffit pas
à se protéger : toute page ouverte dans le même navigateur peut viser
`127.0.0.1`.

- **DNS rebinding** : un nom contrôlé par un tiers résout d'abord vers son
  serveur, puis vers `127.0.0.1` ; la page devient « même origine » pour le
  navigateur et lit les réponses.
- **Requêtes d'une autre page** : `<img>`, `<script>`, `fetch` en mode
  `no-cors`, formulaires… y compris depuis un autre port de `localhost`
  (un autre serveur de développement). Sans lire la réponse, elles peuvent
  modifier le deck ou occuper une place du flux d'évènements.

D'autres serveurs locaux viendront (aperçu, serveur MCP en HTTP, outils de
développement) : il faut une règle unique, pas une garde réinventée à
chaque fois.

## Décision

**Tout serveur HTTP local d'Oku Slides n'accepte que les requêtes qui
viennent de sa propre interface, selon quatre règles vérifiées sur chaque
requête, avant toute route.** La mise en œuvre de référence est
`oku-slides-cli::guard::check` ; un futur serveur la réutilise (ou la
déplace dans un crate partagé) au lieu d'en écrire une autre.

1. **`Host` local** : l'en-tête `Host` est présent une seule fois, bien
   formé, et nomme la boucle locale (`localhost`, `127.0.0.1`, `::1`) ou
   l'adresse d'écoute configurée. Une adresse « partout » (`0.0.0.0`, `::`)
   n'ajoute aucun nom accepté. Cela défait le DNS rebinding : le nom du
   tiers se trahit dans `Host`.
2. **`Sec-Fetch-Site` vaut `same-origin` ou `none`** quand il est présent :
   le navigateur dit lui-même qu'une requête vient d'une autre page
   (`cross-site`, `same-site`), y compris d'un autre port de `localhost`.
3. **`Origin`, quand il est là, est l'origine même de la requête** :
   schéma http(s), même nom et même port que `Host`. C'est le cas de
   l'interface servie directement comme à travers le proxy de Vite, qui
   garde le `Host` du navigateur.
4. **Un POST est `application/json`** : d'une autre origine, le navigateur
   ne l'envoie qu'après une pré-vérification CORS, et le serveur ne donne
   jamais son accord (aucun en-tête `Access-Control-*`).

Une requête refusée reçoit `403` et la règle enfreinte est journalisée.
**Il n'y a pas d'authentification** (ni jeton, ni cookie) : la règle
protège contre le navigateur, pas contre un programme local.

## Conséquences

**Positives**

- Aucune configuration ni secret à transmettre à l'interface : le serveur
  de Vite et l'interface servie directement fonctionnent tels quels.
- Une seule règle, testée (`guard_tests.rs`), pour tous les serveurs
  locaux à venir.

**Négatives (limites connues)**

- **Pas d'authentification.** Un client hors navigateur (curl, script)
  envoie les en-têtes qu'il veut : `Host: 127.0.0.1` sans `Origin` ni
  `Sec-Fetch-Site` passe la garde. Tant que le serveur écoute sur la
  boucle locale, seul un programme de la machine peut le joindre ; s'il
  écoute **hors boucle locale** (`[serve] host = "0.0.0.0"` ou une adresse
  du réseau), **tout client du réseau qui forge `Host` est servi**. `serve`
  l'avertit au démarrage (t-193).
- Un navigateur trop ancien pour envoyer `Sec-Fetch-Site` ne bénéficie que
  des règles 1, 3 et 4.

**Neutres**

- Le jour où une écoute réseau devient un usage voulu (présentation sur un
  autre appareil), il faudra ajouter une authentification (jeton dans
  l'URL ou cookie) : décision à prendre à part, cette règle restant en
  place en plus.

## Alternatives écartées

- **Jeton secret dans l'URL ou un cookie dès maintenant :** protège aussi
  contre le réseau, mais complique le proxy de Vite, les e2e et chaque
  outil qui parle au serveur, pour un serveur qui écoute par défaut sur la
  boucle locale. Reporté tant que l'écoute réseau n'est pas un usage voulu.
- **CORS « ouvert » avec liste d'origines :** CORS règle la lecture des
  réponses, pas l'envoi des requêtes simples ; il ne défait ni le DNS
  rebinding ni les requêtes `no-cors`.
- **Se fier à la seule adresse d'écoute :** la boucle locale est joignable
  par toutes les pages du navigateur.
