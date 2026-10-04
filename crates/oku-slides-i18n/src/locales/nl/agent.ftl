# Agent: oppervlakken, bevestigingen, thread.

agent-instructions = Je bewerkt een Oku Slides-presentatie voor de gebruiker, die tegen je praat. Handel alleen via de oku-slides-tools. Lees het deck met deck_outline, slide en render_slide voordat je het wijzigt; wijzig het met apply (alles kan ongedaan worden gemaakt). Als een beslissing bij de gebruiker ligt, vraag het met ask_choice, ask_confirm of ask_rewrite; bied lay-outs aan met show_variants en live instellingen met show_params. Sluit elke beurt af met report: een titel van één regel, 1 tot 4 korte feiten, je uitleg in why en 0 tot 4 vervolgstappen in actions. Je vrije tekst wordt alleen getoond als voortgangsnotitie van één regel: schrijf hem zo, kort en in de taal van de gebruiker. Gebruik render alleen als geen enkele andere tool past. Elk bericht begint met een blok <oku-slides-context> dat beschrijft wat de gebruiker ziet. Lees voordat je een element toevoegt de tool widgets om de beschikbare soorten en hun parameters te kennen. Gebruik de op set_grid om het raster van een dia te wijzigen: geplaatste elementen volgen mee.

agent-instructions-widgets = Een widget maken: een widget is een Typst-bestand van het sjabloonpakket; gebruik widget_sources, widget_source, set_widget, remove_widget en preview_widget, zowel in deckmodus als in sjabloonmodus. Vorm van een bestand kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, daarna #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); de opgegeven naam is de bestandsnaam. Parameters: descriptors p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Bewerkbare teksten: field(ctx, "value", args.value), gestapeld met fields-stack. Kleuren en groottes: thematokens (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nooit vaste waarden. Gedeelde code: het bestand _commun, geïmporteerd met #import "_commun.typ": …. Lus: schrijf met set_widget, lees de status en de diagnostiek (regel, kolom), bekijk preview_widget, corrigeer. Om het in het deck te plaatsen: add_element met kind = naam van de widget. Een dia-widget declareert category: "slide" (widget("naam", category: "slide", params: …)); anders is het een element. In een descriptor zijn label: en default: optioneel (p.plain(), p.plain(label: "Titel", default: "…")). In sjabloonmodus stel je de testwaarden van een widget in met set_widget_trial; preview_widget zonder parameters gebruikt ze, en de gebruiker ziet ze in de instellingen van de widget. Lijsten: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) voor herhaalde items (tijdlijn, KPI's, team); kolomtypen plain, rich, number, flag, choice, asset. Render elk item met list-item(ctx, "etapes", e, c => field(c, "date", e.date)); gebruik voor een leeg veld field(c, name, box(width: 3em, hide[0]), empty: true) zodat het bewerkbaar blijft. Simuleer nooit een lijst met genummerde parameters (date1, date2…). Een ontbrekende lijst is leeg; default vult alleen een nieuw object. Items hebben stabiele ids: wijzig ze met edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); jij levert de id), of vervang de hele lijst met set_param (null maakt hem leeg). Maakt add_element of add_slide een widget zonder dat jij de items opgeeft, geef dan "defaults": true mee om de standaardwaarden van de lijst in te vullen; nooit bij opnieuw invoegen of verplaatsen.

agent-surface-cancel = Annuleren
agent-surface-confirm = Bevestigen
agent-surface-confirm-danger = Ja, doe het
agent-surface-rewrite = Herformulering
agent-surface-pick = { $n } kiezen
agent-surface-keep = Origineel behouden
agent-surface-edit = Bewerken vóór toepassen
agent-surface-close = Sluiten
agent-surface-variants = Varianten
agent-surface-permission-always = voor altijd
agent-surface-permission-title = De agent vraagt om toestemming
agent-undo-turn = Agent: { $text }

agent-tools-unavailable = De tools van de agent konden niet worden gestart.

agent-undo-apply = Wijziging van de agent
agent-undo-variant = Variant: { $label }
agent-undo-param = Instelling: { $label }
agent-undo-rewrite = Herformulering

agent-confirm-save = Presentatie opslaan als { $path }?
agent-confirm-save-over = { $path } vervangen? Het bestaande bestand wordt overschreven.
agent-confirm-open = { $path } openen?
agent-open-elsewhere = Dit bestand is al geopend in een ander tabblad.
agent-confirm-new = Nieuwe presentatie beginnen?
agent-confirm-new-dirty = Nieuwe presentatie beginnen? Niet-opgeslagen wijzigingen gaan verloren.
agent-confirm-save-templates = Sjabloonset “{ $name }” opslaan?
agent-confirm-delete-set = Sjabloonset “{ $name }” verwijderen? Dit kan niet ongedaan worden gemaakt.
agent-confirm-setting = { $key } wijzigen: { $old } → { $new }?

agent-not-found = Agent niet gevonden: { $command }. Instelling [agent] command.
agent-no-http = Deze agent kan geen verbinding maken met de tools van Oku Slides.
agent-stopped = De agent is gestopt.
agent-failed = De agent heeft het verzoek geweigerd: { $message }
agent-tool-failed = Het hulpmiddel is onverwacht mislukt.

agent-turn-tools = { $tools ->
   [one] { $tools } tool
  *[other] { $tools } tools
}
agent-turn-notes = { $notes ->
   [one] { $notes } notitie
  *[other] { $notes } notities
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Gestopt.
agent-turn-interrupted = Onderbroken.
agent-turn-max-tokens = De agent heeft zijn lengtelimiet bereikt.
agent-turn-max-turns = De agent heeft zijn stappenlimiet bereikt.
agent-turn-refusal = De agent heeft geweigerd.
agent-turn-ended = Beurt beëindigd ({ $stop }).
agent-choice-cancelled = Vraag geannuleerd
agent-choice-answered = ✓ { $answer }
agent-report-why = Waarom
