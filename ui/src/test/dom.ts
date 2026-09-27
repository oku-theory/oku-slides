// DOM de test (happy-dom) pour les tests de rendu React (`*.render.test.tsx`).
//
// Ces fichiers tournent dans leur propre passe de `bun test` (voir le script
// `test` de package.json) : le DOM y est installé une fois, au premier import
// de ce module, et reste en place pour toute la passe. Un fichier de rendu
// importe ce module puis charge React par `await import` : react-dom cherche
// le DOM à son chargement, et Bun n'évalue pas les imports statiques dans
// l'ordre écrit.

import { GlobalRegistrator } from "@happy-dom/global-registrator";

if (!GlobalRegistrator.isRegistered) GlobalRegistrator.register();
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
