pdf-unreadable = Onleesbare pdf
pdf-unsupported-version = Deze pdf is gemaakt met een nieuwere versie van Oku Slides
pdf-corrupted = De bronnen in deze pdf zijn beschadigd
pdf-write = De pdf kon niet worden geschreven

render-compile = De presentatie kon niet worden weergegeven
render-pdf = De pdf-export is mislukt
render-no-page = Deze pagina bestaat niet
render-image = Afbeelding van de dia kon niet worden gemaakt

render-setup = De renderer kon niet starten
render-missing-image = ontbrekende afbeelding
diagnostic-error = fout
diagnostic-warning = waarschuwing
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = tip: { $hint }
render-chart-failed = grafiek kon niet worden getekend

registry-bad-pack-id = Ongeldige id van sjabloonpakket: { $id }
registry-compile = De widgets van het sjabloonpakket konden niet worden gecompileerd
registry-missing = Het sjabloonpakket declareert geen widgets
render-timeout = Het renderen duurde langer dan { $seconds } s en is afgebroken
render-out-of-memory = Het renderen had meer dan { $mib } MiB geheugen nodig en is afgebroken
render-worker-lost = Het renderproces is gestopt
render-frame = Onleesbaar bericht van het renderproces
render-page-too-large = Deze dia is te groot voor de voorvertoning (meer dan { $mib } MiB, vaak door een zeer grote afbeelding). Hij kan nog wel als PDF of PNG worden geëxporteerd.
render-stale = De render is vervangen; probeer het opnieuw
render-pack-disabled = De widgets van pakket “{ $pack }” zijn uitgeschakeld voor deze sessie
render-protocol = Het renderproces hoort bij een andere versie van Oku Slides; start de applicatie opnieuw
render-remote = Het renderproces kon het verzoek niet uitvoeren
registry-schema-remote = Ongeldig widgetregister: { $message }
probe-not-probed = Nog niet gecontroleerd: de tijd voor de widgets van het pakket is op. Dit gebeurt bij de volgende weergave.
probe-too-many-files = Het pakket heeft meer dan { $max } widgetbestanden: dit bestand wordt niet geladen.
probe-too-large = Groter dan { $max } KiB: niet geladen.

import-unreadable = Deze presentatie kan niet worden gelezen
import-not-a-presentation = Dit bestand is geen PowerPoint-presentatie
import-protected = Deze presentatie is versleuteld of in het oude .ppt-formaat: sla haar eerst op als .pptx
import-too-large = Deze presentatie is te groot om te importeren
import-output-too-large = Het resulterende deck zou de toegestane grootte voor een import overschrijden
import-loss-list-levels = Lijstniveaus teruggebracht tot één
import-loss-merged-cells = Samengevoegde cellen gesplitst
import-loss-percent-stacked = 100%-gestapelde grafiek als gestapeld weergegeven
import-loss-chart-format = Getalnotatie { $format } van de grafiek niet behouden
import-loss-run-style = { $effect ->
    [underline] Onderstreping verloren
    [strike] Doorhaling verloren
    [baseline] Superscript of subscript verloren
    [caps] Hoofdletters verloren
    [font] Lettertype van een passage vervangen door dat van het vak
    [tracking] Letterafstand van een passage vervangen door die van het vak
    [weight] Normale passage vet weergegeven zoals het vak
    [slant] Rechte passage cursief weergegeven zoals het vak
   *[highlight] Markering verloren
}
import-loss-transform = Draaiing, spiegeling of bijsnijding verloren
import-loss-color-dropped = De kleur { $color } heeft geen nabije themakleur: themakleur gebruikt
import-loss-text-truncated = Tekst ingekort om binnen de grenzen te blijven
import-loss-table-truncated = Tabel ingekort om binnen de grenzen te blijven
import-loss-chart-truncated = Grafiekgegevens ingekort om binnen de grenzen te blijven
import-loss-opaque = { $reason ->
    [media] Video of geluid niet geïmporteerd (geen vervangende afbeelding)
    [ole] Ingesloten object niet geïmporteerd (geen vervangende afbeelding)
    [math] Vergelijking niet geïmporteerd (geen vervangende afbeelding)
    [smartart] SmartArt niet geïmporteerd (geen vervangende afbeelding)
   *[unknown] Niet-ondersteund object niet geïmporteerd (geen vervangende afbeelding)
}
import-loss-smartart-layout = SmartArt zonder tekening: de tekst is als kaarten geplaatst, de diagramindeling gaat verloren
import-loss-unsupported-image = Afbeelding in { $format }-indeling niet geïmporteerd
import-loss-hidden-slide = Verborgen dia als zichtbaar geïmporteerd
import-loss-effects = Schaduwen, weerspiegelingen en 3D-effecten verloren
import-loss-series-dropped = Een cirkeldiagram toont één reeks: { $kept } behouden, { $dropped } weggelaten
import-loss-repeated-decor = { $slides ->
   [one] Kop- of voettekst “{ $text }” herhaald op { $slides } dia weggelaten: het thema tekent er zelf een
  *[other] Kop- of voettekst “{ $text }” herhaald op { $slides } dia’s weggelaten: het thema tekent er zelf een
}
import-loss-no-slot = Inhoud weggelaten: de gekozen sjabloon heeft er geen plaats voor
import-loss-moved-to-notes = Tekst verplaatst naar de sprekersnotities: de titeldia heeft er geen plaats voor
import-loss-items-dropped = Te veel elementen op de dia: { $dropped } elementen weggelaten
import-loss-page-fitted = Bronpagina van { $width } × { $height } pt is geen 16:9: inhoud geschaald en gecentreerd
import-loss-paragraph-style = Uitlijning, regelafstand, alinea-afstand of inspringing van een alinea gelijkgetrokken met het vak
import-loss-shape-style = { $effect ->
    [gradient] Kleurverloop weergegeven als effen kleur
    [picture] Opvulafbeelding uitgerekt tot het kader van de vorm
    [dash] Stippellijn doorgetrokken weergegeven
    [arrow] Pijlpunten van de lijn niet getekend
   *[geometry] Onbekende vorm weergegeven als rechthoek
}
import-loss-layout-dropped = Te veel lay-outs voor één pakket: ‘{ $layout }’ heeft geen sjabloon; de dia's ervan gebruiken de lege widget en krijgen de decoratie als elementen
import-loss-decor-truncated = { $omitted ->
    [one] Decoratie ingekort om in de grootte van een pakketbestand te passen: { $omitted } element weggelaten
   *[other] Decoratie ingekort om in de grootte van een pakketbestand te passen: { $omitted } elementen weggelaten
}
import-loss-table-style = Tabelstijl benaderd: eigen celachtergronden, randen, kolombreedtes, rijhoogtes of tekstgroottes gaan verloren
import-loss-overflow = Element ‘{ $element }’ steekt buiten de rand van de dia
import-loss-empty-data = Tabel of grafiek zonder gegevens: niets geplaatst
import-reference-unavailable = Geen visuele score: { $cause ->
    [timeout] de referentieweergave duurde te lang
    [memory] de referentieweergave gebruikte te veel geheugen
    [pages] de referentieweergave heeft { $reference ->
        [one] { $reference } pagina
       *[other] { $reference } pagina’s
    } voor { $deck } in de presentatie
    [missing] LibreOffice (soffice) is niet gevonden; geef het pad op in import.soffice
   *[failed] de referentierenderer kon de presentatie niet lezen
}
import-region-suspect = Afwijking terzijde gelegd, een bekende fout van de referentierenderer: { $limit ->
    [autofit] hij verkleint tekst die in zijn vak moet passen wanneer PowerPoint geen schaal heeft opgeslagen
    [tracking] hij tekent tekst met letterafstand breder
    [chart] hij tekent grafieken slechts bij benadering
    [default_font] hij tekent tekst waarvan alleen de standaardinstellingen van de presentatie het lettertype noemen in zijn reservelettertype
    [no_substitute] hij heeft geen vervanger voor dit lettertype en tekent het in een ander
    [smart_art_style] hij tekent SmartArt-tekst in zijn eigen stijl
   *[smart_art] hij tekent SmartArt-tekst te klein
}
import-font-substituted = { $family } vervangen door { $by }, met passende metrieken
import-font-missing = { $family } niet gevonden: de tekst gebruikt { $by }
import-font-restricted = { $family }: de licentie verbiedt sommige stijlen in de presentatie in te sluiten; tekst in die stijlen gebruikt de overige stijlen
import-font-restricted-substituted = { $family } mag niet in de presentatie worden ingesloten (licentie): vervangen door { $by }, met passende metrieken
import-font-restricted-missing = { $family } mag niet in de presentatie worden ingesloten (licentie): de tekst gebruikt { $by }
import-font-too-large = { $family }: bestanden te groot om in het deck in te sluiten; de tekst gebruikt { $by }
import-font-missing-no-fallback = { $family } niet gevonden, net als het hoofdtekstlettertype van het thema: de tekst wordt weergegeven met het standaardlettertype van de engine
import-font-too-large-no-fallback = { $family }: bestanden te groot om in het deck in te sluiten, en het hoofdtekstlettertype van het thema is ook niet beschikbaar: de tekst wordt weergegeven met het standaardlettertype van de engine
import-font-restricted-no-fallback = { $family } mag niet in de presentatie worden ingesloten (licentie), en het hoofdtekstlettertype van het thema is ook niet beschikbaar: de tekst wordt weergegeven met het standaardlettertype van de engine
import-warning-effect-ignored = Effect niet geïmporteerd: { $detail }
import-warning-approximated = Benaderd weergegeven: { $detail }
import-warning-unreadable-item = Onleesbaar element overgeslagen: { $detail }
import-warning-unreadable-part = Onleesbaar onderdeel: { $detail }
import-warning-missing-part = Ontbrekend onderdeel: { $detail }
import-warning-unsupported-chart = Niet-ondersteunde grafiek: { $detail }
import-warning-unsupported-image = Afbeeldingsindeling niet ondersteund: { $detail }
import-warning-oversized-image = Afbeelding te groot om te importeren (aantal pixels): { $detail }
import-warning-external-image = Gekoppelde afbeelding buiten het bestand, niet geïmporteerd: { $detail }
import-warning-unsafe-svg = SVG-afbeelding ontdaan van onveilige inhoud, of weggelaten: { $detail }
import-warning-unknown-table-style = Onbekende tabelstijl: { $detail }
import-warning-unsafe-link = Koppeling geweigerd: { $detail }
import-warning-nesting-too-deep = Nesting te diep, afgekapt: { $detail }
import-warning-font-embedded-unreadable = Ingesloten lettertype onleesbaar: { $detail }
import-warning-truncated = Te veel waarschuwingen: de rest wordt weggelaten
