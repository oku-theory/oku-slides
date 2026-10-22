cli-reading = lettura di { $path }
cli-invalid-deck = presentazione non valida: { $path }
cli-deck-valid = presentazione valida: { $slides } diapositive
cli-deck-issue = problema: { $issue }
cli-rendered = { $pages } pagine → { $out } (compilazione { $compile }, SVG { $svg })
cli-asset-added = risorsa { $path } → { $id }
cli-saved = { $path }: { $pages } pagine, PDF { $pdf } KB, sorgenti { $sources } KB
cli-opened = { $path }: { $slides } diapositive, { $assets } risorse, { $fonts } font
cli-extracted = estratto in { $dir }
cli-stripped = { $path }: copia senza sorgenti
cli-pdf-modified = il PDF è stato modificato da un altro strumento dopo Oku Slides
cli-iterations = --iterations deve essere > 0
cli-no-block = la presentazione non ha alcun blocco da modificare
cli-blob-missing = blob delle sorgenti non trovato
cli-roundtrip-mismatch = la presentazione riletta è diversa da quella salvata
cli-bench-world = costruzione del mondo (font)      { $time }
cli-bench-first = prima compilazione                { $time }  ({ $pages } pagine)
cli-bench-same = ricompilazione senza modifiche    { $time }
cli-bench-edited-text = Testo modificato { $n }
cli-bench-edit = ricompilazione dopo set_text      { $time }
cli-bench-svg = SVG di ogni diapositiva           { $time }  ({ $each } per diapositiva)
cli-bench-pdf = esportazione PDF                  { $time }  ({ $size } KB)
cli-bench-embed = incorporamento del blob sorgenti  { $time }
cli-bench-extract = rilettura del blob                { $time }
cli-bench-roundtrip = andata e ritorno presentazione    identica
cli-bench-history = passo della cronologia          { $mean } ms in media, p50 { $p50 } ms
cli-serving = Oku Slides (dev) su http://{ $host }:{ $port }
cli-serving-token = apri { $url } (il token vale fino all'arresto del server)
cli-models-dir = modelli di dettatura: { $dir }
cli-stopped = server arrestato
cli-not-found = non trovato
cli-bad-request = richiesta non valida: { $reason }
cli-sse-full = troppi flussi di eventi già aperti
cli-forbidden = richiesta rifiutata: non proviene dall'interfaccia locale
cli-unauthorized = richiesta rifiutata: token di accesso mancante o errato
cli-exposed = in ascolto su { $host }: il server è raggiungibile dalla rete; vengono serviti solo i client che presentano il token di questo avvio. Lascia [serve] host su 127.0.0.1 se la rete non ti serve
cli-imported = { $path }: { $slides ->
       [one] { $slides } diapositiva
      *[other] { $slides } diapositive
    }, { $pages ->
       [one] { $pages } pagina
      *[other] { $pages } pagine
    }, { $losses ->
       [one] { $losses } perdita
      *[other] { $losses } perdite
    }, { $requests ->
       [one] { $requests } richiesta di widget
      *[other] { $requests } richieste di widget
    }
cli-import-write-failed = impossibile scrivere la presentazione importata { $path }
cli-import-hifi-no-target = --pack e --theme valgono solo in modalità retrofit: l'importazione ad alta fedeltà (--mode hifi) genera un proprio pacchetto e tema
cli-import-reference-hifi-only = --reference appartiene alla modalità hifi: un'importazione retrofit non è confrontata con un rendering di riferimento
cli-import-scored = Scarto visivo da { $witness }: { $mean } in media, { $above ->
    [0] nessuna slide
    [one] { $above } slide
   *[other] { $above } slide
} oltre { $threshold }
cli-import-unscored = { $reason }

cli-about = Genera e archivia presentazioni Oku Slides
cli-arg-fonts = Cartella di font aggiuntiva (TTF/OTF); predefinita: paths.fonts nelle impostazioni
cli-cmd-check = Verifica una presentazione
cli-arg-deck = Presentazione (JSON)
cli-cmd-render = Genera ogni diapositiva in SVG, con i riquadri degli elementi
cli-arg-input = File deck.json o .oku-slides.pdf
cli-arg-out = Cartella di output
cli-arg-theme = Sostituisci il tema della presentazione: uno incluso o della libreria, per nome
cli-cmd-save = Salva una presentazione come .oku-slides.pdf (PDF + blob delle sorgenti)
cli-arg-output = File da scrivere
cli-arg-assets = Cartella delle risorse: ogni file diventa una risorsa sha256:…
cli-arg-embed-fonts = Incorpora nelle sorgenti anche i font di --fonts
cli-cmd-open = Riapri un .oku-slides.pdf: estrai la presentazione e le sue risorse
cli-arg-extract = Cartella in cui scrivere deck.json, assets/ e fonts/
cli-cmd-strip = Copia un .oku-slides.pdf senza il blob delle sorgenti
cli-cmd-bench = Misura i tempi di compilazione ed esportazione
cli-arg-iterations = Misurazioni per fase
cli-cmd-schema = Stampa lo schema JSON delle operazioni
cli-cmd-serve = Server di sviluppo: l’interfaccia in un browser, senza Tauri
cli-arg-serve-input = Presentazione da aprire (.oku-slides.pdf o .json)
cli-arg-port = Porta; predefinita: serve.port nelle impostazioni
cli-arg-ui = Cartella dell’interfaccia compilata (ui/dist)
cli-cmd-import = Importa una presentazione PowerPoint (.pptx) come .oku-slides.pdf
cli-arg-pptx = Presentazione PowerPoint (.pptx)
cli-arg-mode = retrofit: il contenuto su un pacchetto e un tema esistenti; hifi: copia fedele, su un pacchetto e un tema generati apposta
cli-arg-pack = Pacchetto di layout per retrofit (spazio dei nomi, id o cartella); ripetibile, predefinito essentiel
cli-arg-import-theme = Tema di destinazione: uno incluso o della libreria, per id o nome; predefinito: il tema predefinito
cli-arg-report = Scrivi anche il rapporto di importazione (JSON): perdite, sostituzioni, copertura
cli-arg-no-system-fonts = In modalità hifi, non cercare i caratteri della presentazione tra quelli di sistema
cli-arg-reference = In modalità hifi, confrontare ogni slide con un rendering di riferimento della presentazione: auto (predefinito: [import] witness, altrimenti office2pdf), office2pdf, soffice (LibreOffice) o no (none)
