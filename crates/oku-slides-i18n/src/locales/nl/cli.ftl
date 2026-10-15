cli-reading = { $path } lezen
cli-invalid-deck = ongeldige presentatie: { $path }
cli-deck-valid = geldige presentatie: { $slides } dia's
cli-deck-issue = probleem: { $issue }
cli-rendered = { $pages } pagina's → { $out } (compilatie { $compile }, SVG { $svg })
cli-asset-added = bron { $path } → { $id }
cli-saved = { $path }: { $pages } pagina's, pdf { $pdf } KB, bronnen { $sources } KB
cli-opened = { $path }: { $slides } dia's, { $assets } bronnen, { $fonts } lettertypen
cli-extracted = uitgepakt naar { $dir }
cli-stripped = { $path }: kopie zonder bronnen
cli-pdf-modified = de pdf is na Oku Slides door een ander programma gewijzigd
cli-iterations = --iterations moet > 0 zijn
cli-no-block = de presentatie heeft geen blok om te bewerken
cli-blob-missing = bronnenblob niet gevonden
cli-roundtrip-mismatch = de opnieuw gelezen presentatie wijkt af van de opgeslagen versie
cli-bench-world = omgeving opbouwen (lettertypen)  { $time }
cli-bench-first = eerste compilatie                { $time }  ({ $pages } pagina's)
cli-bench-same = hercompilatie zonder wijziging   { $time }
cli-bench-edited-text = Bewerkte tekst { $n }
cli-bench-edit = hercompilatie na set_text        { $time }
cli-bench-svg = SVG van elke dia                 { $time }  ({ $each } per dia)
cli-bench-pdf = pdf-export                       { $time }  ({ $size } KB)
cli-bench-embed = bronnenblob insluiten            { $time }
cli-bench-extract = blob teruglezen                  { $time }
cli-bench-roundtrip = heen-en-terug presentatie        identiek
cli-bench-history = stap in de geschiedenis          { $mean } ms gemiddeld, p50 { $p50 } ms
cli-serving = Oku Slides (dev) op http://{ $host }:{ $port }
cli-serving-token = open { $url } (het token is geldig tot de server stopt)
cli-models-dir = dicteermodellen: { $dir }
cli-stopped = server gestopt
cli-not-found = niet gevonden
cli-bad-request = ongeldig verzoek: { $reason }
cli-sse-full = te veel gebeurtenisstromen al geopend
cli-forbidden = verzoek geweigerd: het komt niet van de lokale interface
cli-unauthorized = verzoek geweigerd: toegangstoken ontbreekt of is onjuist
cli-exposed = luistert op { $host }: de server is bereikbaar vanaf het netwerk; alleen clients die het token van deze start tonen, worden bediend. Houd [serve] host op 127.0.0.1 als u het netwerk niet nodig hebt
cli-imported = { $path }: { $slides ->
       [one] { $slides } dia
      *[other] { $slides } dia's
    }, { $pages ->
       [one] { $pages } pagina
      *[other] { $pages } pagina's
    }, { $losses ->
       [one] { $losses } verlies
      *[other] { $losses } verliezen
    }, { $requests ->
       [one] { $requests } widgetverzoek
      *[other] { $requests } widgetverzoeken
    }
cli-import-label = PowerPoint-import
cli-import-pack-required = --pack is verplicht in de modus retrofit: de id, map of naam van een sjabloonpakket
cli-import-unknown-pack = onbekend sjabloonpakket: { $pack }
cli-import-bad-pack = onleesbaar sjabloonpakket { $pack }: de pack.json beschrijft geen sjabloonpakket
cli-import-write-failed = kon de geïmporteerde presentatie { $path } niet schrijven
cli-import-hifi-no-target = --pack en --theme horen bij de modus retrofit: import met hoge getrouwheid (--mode hifi) maakt een eigen pakket en thema
cli-import-probe-mismatch = het gemaakte pakket komt bij het peilen niet overeen met de declaratie (widget { $widget }): een fout in de import
cli-import-probe-limit = het gegenereerde pakket overschrijdt de grenzen van de controle (widget { $widget }): verhoog max_widgets, max_widget_kib of probe_budget_ms in de sectie [render] van de configuratie
cli-import-reference-hifi-only = --reference hoort bij de hifi-modus: een retrofit-import wordt niet met een referentieweergave vergeleken
cli-import-bad-threshold = score_threshold in de sectie [import] moet een getal van nul of meer zijn, niet { $value }
cli-import-scored = Visueel verschil met { $witness }: gemiddeld { $mean }, { $above ->
    [0] geen dia
    [one] { $above } dia
   *[other] { $above } dia’s
} boven { $threshold }
cli-import-unscored = { $reason }

cli-about = Oku Slides-presentaties weergeven en opslaan
cli-arg-fonts = Extra lettertypemap (TTF/OTF); standaard: paths.fonts in de instellingen
cli-cmd-check = Een presentatie controleren
cli-arg-deck = Presentatie (JSON)
cli-cmd-render = Elke dia naar SVG renderen, met de elementkaders
cli-arg-input = deck.json- of .oku-slides.pdf-bestand
cli-arg-out = Uitvoermap
cli-arg-theme = Het thema van de presentatie vervangen: een ingebouwd thema of een uit de bibliotheek, op naam
cli-cmd-save = Een presentatie opslaan als .oku-slides.pdf (pdf + bronnenblob)
cli-arg-output = Te schrijven bestand
cli-arg-assets = Bronnenmap: elk bestand wordt een sha256:…-bron
cli-arg-embed-fonts = Ook de lettertypen van --fonts in de bronnen insluiten
cli-cmd-open = Een .oku-slides.pdf opnieuw openen: de presentatie en bronnen uitpakken
cli-arg-extract = Map waarin deck.json, assets/ en fonts/ worden geschreven
cli-cmd-strip = Een .oku-slides.pdf kopiëren zonder bronnenblob
cli-cmd-bench = Compilatie- en exporttijden meten
cli-arg-iterations = Metingen per stap
cli-cmd-schema = Het JSON-schema van de bewerkingen tonen
cli-cmd-serve = Ontwikkelserver: de interface in een browser, zonder Tauri
cli-arg-serve-input = Te openen presentatie (.oku-slides.pdf of .json)
cli-arg-port = Poort; standaard: serve.port in de instellingen
cli-arg-ui = Map van de gebouwde interface (ui/dist)
cli-cmd-import = Een PowerPoint-presentatie (.pptx) importeren als .oku-slides.pdf
cli-arg-pptx = PowerPoint-presentatie (.pptx)
cli-arg-mode = retrofit: de inhoud op een bestaand sjabloonpakket en thema; hifi: getrouwe kopie, op een daarvoor gemaakt pakket en thema
cli-arg-pack = Doelpakket: id, pakketmap (pack.json, widgets/) of naam
cli-arg-import-theme = Doelthema: een ingebouwd thema of een uit de bibliotheek, op id of naam; standaard: het standaardthema
cli-arg-report = Ook het importrapport (JSON) schrijven: verliezen, vervangingen, dekking
cli-arg-no-system-fonts = In modus hifi de lettertypen van de presentatie niet tussen de systeemlettertypen zoeken
cli-arg-reference = In hifi-modus elke dia vergelijken met een referentieweergave van de presentatie (auto, standaard) of niet (none)
