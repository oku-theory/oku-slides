cli-reading = reading { $path }
cli-invalid-deck = invalid deck: { $path }
cli-deck-valid = valid deck: { $slides } slides
cli-deck-issue = issue: { $issue }
cli-rendered = { $pages } pages → { $out } (compilation { $compile }, SVG { $svg })
cli-asset-added = asset { $path } → { $id }
cli-saved = { $path }: { $pages } pages, PDF { $pdf } KB, sources { $sources } KB
cli-opened = { $path }: { $slides } slides, { $assets } assets, { $fonts } fonts
cli-extracted = extracted to { $dir }
cli-stripped = { $path }: copy without sources
cli-pdf-modified = the PDF was changed by another tool after Oku Slides
cli-iterations = --iterations must be > 0
cli-no-block = the deck has no block to edit
cli-blob-missing = sources blob not found
cli-roundtrip-mismatch = the reread deck differs from the saved one
cli-bench-world = building the world (fonts)       { $time }
cli-bench-first = first compilation               { $time }  ({ $pages } pages)
cli-bench-same = recompilation without change    { $time }
cli-bench-edited-text = Edited text { $n }
cli-bench-edit = recompilation after set_text    { $time }
cli-bench-svg = SVG of every slide              { $time }  ({ $each } per slide)
cli-bench-pdf = PDF export                      { $time }  ({ $size } KB)
cli-bench-embed = embedding the sources blob      { $time }
cli-bench-extract = reading the blob back           { $time }
cli-bench-roundtrip = deck round trip                 identical
cli-bench-history = history step                       { $mean } ms on average, p50 { $p50 } ms
cli-serving = Oku Slides (dev) at http://{ $host }:{ $port }
cli-serving-token = open { $url } (the token is valid until the server stops)
cli-models-dir = dictation models: { $dir }
cli-stopped = server stopped
cli-not-found = not found
cli-bad-request = invalid request: { $reason }
cli-sse-full = too many event streams already open
cli-forbidden = request refused: it does not come from the local interface
cli-unauthorized = request refused: missing or wrong access token
cli-exposed = listening on { $host }: the server is reachable from the network; only clients that present this launch's token are served. Keep [serve] host on 127.0.0.1 unless you need the network
cli-imported = { $path }: { $slides ->
       [one] { $slides } slide
      *[other] { $slides } slides
    }, { $pages ->
       [one] { $pages } page
      *[other] { $pages } pages
    }, { $losses ->
       [one] { $losses } loss
      *[other] { $losses } losses
    }, { $requests ->
       [one] { $requests } widget request
      *[other] { $requests } widget requests
    }
cli-import-write-failed = could not write the imported deck { $path }
cli-import-hifi-no-target = --pack and --theme belong to retrofit mode: the high-fidelity import (--mode hifi) generates its own pack and theme
cli-import-reference-hifi-only = --reference belongs to hifi mode: a retrofit import is not compared with a reference render
cli-import-scored = Visual gap to { $witness }: { $mean } on average, { $above ->
    [0] no slide
    [one] { $above } slide
   *[other] { $above } slides
} above { $threshold }
cli-import-unscored = { $reason }

cli-about = Render and store Oku Slides decks
cli-arg-fonts = Extra font folder (TTF/OTF); default: paths.fonts in the settings
cli-cmd-check = Check a deck
cli-arg-deck = Deck (JSON)
cli-cmd-render = Render each slide to SVG, with the element boxes
cli-arg-input = deck.json or .oku-slides.pdf file
cli-arg-out = Output folder
cli-arg-theme = Replace the deck's theme: a built-in or library theme, by namespace, id or name
cli-cmd-save = Save a deck as .oku-slides.pdf (PDF + sources blob)
cli-arg-output = File to write
cli-arg-assets = Asset folder: each file becomes a sha256:… asset
cli-arg-embed-fonts = Also embed the --fonts fonts in the sources
cli-cmd-open = Reopen an .oku-slides.pdf: extract the deck and its assets
cli-arg-extract = Folder to write deck.json, assets/ and fonts/ into
cli-cmd-strip = Copy an .oku-slides.pdf without its sources blob
cli-cmd-bench = Measure compilation and export times
cli-arg-iterations = Measurements per step
cli-cmd-schema = Print the JSON schema of the operations
cli-cmd-serve = Development server: the interface in a browser, without Tauri
cli-arg-serve-input = Deck to open (.oku-slides.pdf or .json)
cli-arg-port = Port; default: serve.port in the settings
cli-arg-ui = Folder of the built interface (ui/dist)
cli-cmd-import = Import a PowerPoint presentation (.pptx) as an .oku-slides.pdf deck
cli-arg-pptx = PowerPoint presentation (.pptx)
cli-arg-mode = retrofit: the content on an existing pack and theme; hifi: faithful copy, on a pack and theme generated for it
cli-arg-pack = Layout pack for retrofit (namespace, id or folder); repeatable, default essentiel
cli-arg-import-theme = Target theme: a built-in or library theme, by namespace, id or name; default: the default theme
cli-arg-report = Also write the import report (JSON): losses, fallbacks, coverage
cli-arg-no-system-fonts = In hifi mode, do not look for the presentation's fonts among the system fonts
cli-arg-reference = In hifi mode, compare each slide with a reference render of the presentation: auto (the default: [import] witness, office2pdf unless set), office2pdf, soffice (LibreOffice) or not (none)
