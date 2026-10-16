pdf-unreadable = PDF illeggibile
pdf-unsupported-version = Questo PDF è stato scritto da una versione più recente di Oku Slides
pdf-corrupted = Le sorgenti di questo PDF sono danneggiate
pdf-write = Impossibile scrivere il PDF

render-compile = Impossibile generare la presentazione
render-pdf = Esportazione PDF non riuscita
render-no-page = Questa pagina non esiste
render-setup = Impossibile avviare il motore di rendering
render-image = Impossibile generare l'immagine della diapositiva
render-missing-image = immagine mancante
diagnostic-error = errore
diagnostic-warning = avviso
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = suggerimento: { $hint }
render-chart-failed = impossibile disegnare il grafico

registry-bad-pack-id = Identificatore del pacchetto di modelli non valido: { $id }
registry-compile = Impossibile compilare i widget del pacchetto di modelli
registry-missing = Il pacchetto di modelli non dichiara i propri widget
render-timeout = Il rendering ha impiegato più di { $seconds } s ed è stato interrotto
render-out-of-memory = Il rendering ha richiesto più di { $mib } MiB di memoria ed è stato interrotto
render-worker-lost = Il processo di rendering si è arrestato
render-frame = Messaggio illeggibile dal processo di rendering
render-page-too-large = Questa slide è troppo grande per l'anteprima (più di { $mib } MiB, spesso a causa di un'immagine molto grande). Si può comunque esportare in PDF o PNG.
render-stale = Il rendering è stato sostituito; riprova
render-pack-disabled = I widget del pacchetto «{ $pack }» sono disattivati per questa sessione
render-protocol = Il processo di rendering appartiene a un'altra versione di Oku Slides; riavvia l'applicazione
render-remote = Il processo di rendering non è riuscito a completare la richiesta
registry-schema-remote = Registro dei widget non valido: { $message }
probe-not-probed = Non ancora verificato: il tempo concesso ai widget del pacchetto è scaduto. Verrà verificato al prossimo rendering.
probe-too-many-files = Il pacchetto ha più di { $max } file di widget: questo non viene caricato.
probe-too-large = Più di { $max } KiB: non caricato.

import-unreadable = Impossibile leggere questa presentazione
import-not-a-presentation = Questo file non è una presentazione PowerPoint
import-protected = Questa presentazione è crittografata o nel vecchio formato .ppt: salvala prima come .pptx
import-too-large = Questa presentazione è troppo grande per essere importata
import-output-too-large = La presentazione risultante supererebbe la dimensione consentita per un'importazione
import-loss-list-levels = Livelli dell'elenco ridotti a uno
import-loss-merged-cells = Celle unite separate
import-loss-percent-stacked = Grafico in pila al 100% mostrato in pila
import-loss-chart-format = Formato numerico { $format } del grafico non mantenuto
import-loss-run-style = { $effect ->
    [underline] Sottolineatura persa
    [strike] Barrato perso
    [baseline] Apice o pedice perso
    [caps] Maiuscole perse
    [font] Carattere di un passaggio sostituito da quello della casella
    [tracking] Spaziatura dei caratteri di un passaggio sostituita da quella della casella
    [weight] Passaggio normale reso in grassetto come la casella
    [slant] Passaggio dritto reso in corsivo come la casella
   *[highlight] Evidenziazione persa
}
import-loss-transform = Rotazione, capovolgimento o ritaglio persi
import-loss-color-dropped = Il colore { $color } non ha un colore vicino nel tema: usato un colore del tema
import-loss-text-truncated = Testo accorciato per rispettare i limiti
import-loss-table-truncated = Tabella accorciata per rispettare i limiti
import-loss-chart-truncated = Dati del grafico accorciati per rispettare i limiti
import-loss-opaque = { $reason ->
    [media] Video o audio non importato (nessuna immagine sostitutiva)
    [ole] Oggetto incorporato non importato (nessuna immagine sostitutiva)
    [math] Equazione non importata (nessuna immagine sostitutiva)
    [smartart] SmartArt non importato (nessuna immagine sostitutiva)
   *[unknown] Oggetto non supportato non importato (nessuna immagine sostitutiva)
}
import-loss-smartart-layout = SmartArt senza il disegno: il testo è disposto in schede, il layout del diagramma è perso
import-loss-unsupported-image = Immagine in formato { $format } non importata
import-loss-hidden-slide = Diapositiva nascosta importata come visibile
import-loss-effects = Ombre, riflessi ed effetti 3D persi
import-loss-series-dropped = Un grafico a torta mostra una sola serie: { $kept } mantenuta, { $dropped } scartate
import-loss-repeated-decor = { $slides ->
   [one] Intestazione o piè di pagina «{ $text }» ripetuto su { $slides } diapositiva omesso: il tema disegna i propri
  *[other] Intestazione o piè di pagina «{ $text }» ripetuto su { $slides } diapositive omesso: il tema disegna i propri
}
import-loss-no-slot = Contenuto omesso: il modello scelto non ha posto per esso
import-loss-moved-to-notes = Testo spostato nelle note del relatore: la copertina non ha posto per esso
import-loss-items-dropped = Troppi elementi nella diapositiva: { $dropped } elementi omessi
import-loss-page-fitted = La pagina di origine ({ $width } × { $height } pt) non è 16:9: contenuto ridimensionato e centrato
import-loss-paragraph-style = Allineamento, interlinea, spaziatura o rientro di un paragrafo uniformati a quelli della casella
import-loss-shape-style = { $effect ->
    [gradient] Sfumatura resa come colore pieno
    [picture] Immagine di riempimento adattata alla cornice della forma
    [dash] Linea tratteggiata resa continua
    [arrow] Punte di freccia della linea non disegnate
   *[geometry] Forma sconosciuta resa come rettangolo
}
import-loss-layout-dropped = Troppi layout per un pacchetto: «{ $layout }» non ha un modello; le sue slide usano il widget vuoto e ricevono la sua decorazione come elementi
import-loss-decor-truncated = { $omitted ->
    [one] Decorazione tagliata per stare nella dimensione di un file del pacchetto: { $omitted } elemento omesso
   *[other] Decorazione tagliata per stare nella dimensione di un file del pacchetto: { $omitted } elementi omessi
}
import-loss-table-style = Stile della tabella approssimato: si perdono sfondi delle celle, bordi, larghezze delle colonne, altezze delle righe o dimensioni del testo propri
import-loss-overflow = L’elemento «{ $element }» supera il bordo della slide
import-loss-empty-data = Tabella o grafico senza dati: non viene inserito nulla
import-reference-unavailable = Nessun punteggio visivo: { $cause ->
    [timeout] il rendering di riferimento ha richiesto troppo tempo
    [memory] il rendering di riferimento ha usato troppa memoria
    [pages] il rendering di riferimento ha { $reference ->
        [one] { $reference } pagina
       *[other] { $reference } pagine
    } per { $deck } nella presentazione
    [missing] LibreOffice (soffice) non trovato; indicarne il percorso in import.soffice
   *[failed] il renderer di riferimento non ha potuto leggere la presentazione
}
import-font-substituted = { $family } sostituito da { $by }, con metriche compatibili
import-font-missing = { $family } non trovato: il testo usa { $by }
import-font-restricted = { $family }: la licenza vieta di incorporare alcune sue varianti nella presentazione; il testo in quegli stili usa le altre varianti
import-font-restricted-substituted = { $family } non può essere incorporato nella presentazione (licenza): sostituito da { $by }, con metriche compatibili
import-font-restricted-missing = { $family } non può essere incorporato nella presentazione (licenza): il testo usa { $by }
import-font-too-large = { $family }: file troppo grandi per essere incorporati nel deck; il testo usa { $by }
import-font-missing-no-fallback = { $family } non trovato, né il font del corpo del tema: il testo è reso con il font predefinito del motore
import-font-too-large-no-fallback = { $family }: file troppo grandi per essere incorporati nel deck, e nemmeno il font del corpo del tema è disponibile: il testo è reso con il font predefinito del motore
import-font-restricted-no-fallback = { $family } non può essere incorporato nella presentazione (licenza), e nemmeno il font del corpo del tema è disponibile: il testo è reso con il font predefinito del motore
import-warning-effect-ignored = Effetto non importato: { $detail }
import-warning-approximated = Resa approssimata: { $detail }
import-warning-unreadable-item = Elemento illeggibile ignorato: { $detail }
import-warning-unreadable-part = Parte illeggibile: { $detail }
import-warning-missing-part = Parte mancante: { $detail }
import-warning-unsupported-chart = Grafico non supportato: { $detail }
import-warning-unsupported-image = Formato immagine non supportato: { $detail }
import-warning-oversized-image = Immagine troppo grande da importare (numero di pixel): { $detail }
import-warning-external-image = Immagine collegata esterna al file, non importata: { $detail }
import-warning-unsafe-svg = Immagine SVG ripulita da contenuti non sicuri, o scartata: { $detail }
import-warning-unknown-table-style = Stile di tabella sconosciuto: { $detail }
import-warning-unsafe-link = Collegamento rifiutato: { $detail }
import-warning-nesting-too-deep = Annidamento troppo profondo, troncato: { $detail }
import-warning-font-embedded-unreadable = Carattere incorporato illeggibile: { $detail }
import-warning-truncated = Troppi avvisi: i successivi sono omessi
