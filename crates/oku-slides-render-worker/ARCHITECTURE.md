# oku-slides-render-worker

Rendu Typst dans un processus de travail, borné en temps et en mémoire : un
rendu qui boucle ou qui consomme trop se tue sans emporter l'application.

## Types clés

- `RenderWorker` (`client.rs`) : le lien partagé (`Arc<Mutex<Link>>`) ;
  `is_faulty`, `slot`. Un pack qui fait échouer
  le rendu (délai, mémoire, arrêt) devient fautif et n'est plus exécuté.
- `probe_pack` (`pack_probe.rs`) : sonde par fichier, jamais d'accusation (un délai = un
  diagnostic), gardée par `probe_key`, budget total `timeout_ms` par appel ; `trial_slot` : emplacement d'aperçu qui n'accuse aucun pack.
- `RenderSlot` (`client.rs`) : un emplacement (`set_widgets`, `set_language`,
  `render`) ; son `Drop` met `CloseSlot` en file.
- `RemoteCompiled` (`client.rs`) : une génération ; `layout`, `page_hash` locaux,
  `svg`/`png`/`pdf` par requête ; son `Drop` met `Release` en file. La file est
  envoyée avant la requête suivante : un `Drop` ne prend jamais le verrou du lien.
- `Link` (`link.rs`, interne) : transport (sous-processus ou fil), délai, RSS
  relevée toutes les 100 ms, relance et rejeu de `ReplayState`. Le fils reçoit
  ses bornes (`--timeout-ms`, `--memory-bytes`) et `OKU_SLIDES_RENDER_WORKER=1` :
  `Platform::load` y choisit `InProcess`, un oubli d'`exit_if_worker` ne relance pas en chaîne.
- `serve` (`serve.rs`) : la boucle du travailleur ; `exit_if_worker` l'appelle
  quand le binaire est lancé avec `--render-worker`, sous le chien de garde
  (`watchdog.rs`) : requête > 2 × délai, RSS > limite pendant 1 s, parent disparu → sortie 3.
- `Request`/`Response` (`protocol.rs`), trames (`frame.rs`) : en-tête JSON et
  charge brute, bornés par `max_frame_mib` des deux côtés (à l'envoi, refus
  `Frame` sans rien écrire ; à la lecture, avant d'allouer).

## Point d'extension

Une nouvelle sortie d'une page : une requête dans `protocol.rs`, sa branche
dans `serve.rs`, une méthode sur `RemoteCompiled`.

## Dépendances

- `oku-slides-render` : `Renderer`, erreurs sérialisables, sonde du registre.
- `oku-slides-core` (`Deck`, `WidgetRegistry`), `oku-slides-i18n` (langue).
- `oku-slides-platform` : `RenderLaunch`, réglages `[render]`, journaux.
- `serde`, `serde_json` (`float_roundtrip` : la mise en page au bit près) ;
  `sha2`, `hex` : empreintes des polices et des packs.
- `snafu` : erreurs ; `tracing` : journaux ; `sysinfo` : RSS du travailleur.
