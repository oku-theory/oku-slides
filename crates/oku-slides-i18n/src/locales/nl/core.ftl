op-slide-not-found = Dia niet gevonden: { $subject }
op-element-not-found = Element niet gevonden: { $subject }
op-duplicate-id = Id al in gebruik: { $subject }
op-index-out-of-range = Positie buiten bereik
op-invalid-value = Ongeldige waarde voor { $subject }
op-unknown-widget = Onbekende widget: { $subject }
op-unknown-param = Onbekende parameter: { $subject }
op-param-type = Ongeldige waarde voor { $subject }
op-item-not-found = Geen item { $subject }
op-list-full = De lijst { $subject } is vol (maximaal { $max ->
   [one] { $max } item
  *[other] { $max } items
})
op-list-minimum = De lijst { $subject } heeft minstens { $min ->
   [one] { $min } item
  *[other] { $min } items
} nodig
op-list-cell-too-long = Een cel van de lijst { $subject } is te lang (kolom { $column })
op-list-id-too-long = Een item-id van de lijst { $subject } is te lang
op-list-too-many-items = De lijst { $subject } heeft te veel items (maximaal { $max })
op-slot-rejects = { $subject } is hier niet toegestaan
op-wrong-category = { $subject } kan hier niet worden gebruikt
op-choice-out-of-range = Waarde niet toegestaan voor { $subject }
transaction-refused = Bewerking { $index } geweigerd: { $reason }

sources-unreadable = Onleesbare bronnen
sources-not-sources = Dit is geen Oku Slides-bronarchief
sources-unsupported-version = Deze bronnen zijn gemaakt met een nieuwere versie van Oku Slides
sources-bad-name = Geweigerde bestandsnaam in de bronnen
sources-missing-deck = De presentatie ontbreekt in de bronnen
sources-too-large = De bronnen in dit bestand zijn te groot om te openen
sources-too-deep = Deze presentatie nest elementen te diep

issue-empty-slide-id = lege dia-id
issue-duplicate-slide-id = dubbele dia-id
issue-duplicate-element-id = dubbele element-id
issue-empty-element-id = lege id
issue-cell-outside-grid = cel buiten het raster
issue-empty-frame = kader zonder afmetingen
issue-free-without-frame = vrij element zonder kader
issue-unknown-widget = onbekende widget: { $kind }
issue-wrong-category = { $kind } kan hier niet worden gebruikt
issue-unknown-param = onbekende parameter: { $param }
issue-param-type = ongeldige waarde voor { $param }
issue-choice-out-of-range = waarde niet toegestaan voor { $param }
issue-slot-rejects = { $kind } wordt hier niet geaccepteerd
issue-placed-in-flow = element met een kader in een opgemaakte lijst
issue-unknown-bind = { $param } is gekoppeld aan een onbekende deckinstelling
issue-slot-too-deep = elementen zijn te diep genest
issue-missing-asset = ontbrekende bron: { $asset }
issue-pack-version-mismatch = gewijzigde pakketversie verworpen: { $version }
issue-pack-version-missing = pakketversie ontbreekt in de bronnen: { $version }
issue-theme-version-mismatch = gewijzigde themaversie verworpen: { $version }
issue-theme-version-missing = themaversie ontbreekt in de bronnen: { $version }
issue-unsupported-link = niet-ondersteunde link: { $link }
issue-highlight-zero = markeerkleuren worden genummerd vanaf 1
issue-unreadable-element = onleesbaar element, apart gezet
issue-unreadable-item = onleesbaar lijstitem, apart gezet
issue-data-truncated = gegevens te groot, bij het lezen ingekort
issue-unreadable-slide = onleesbare dia, apart gezet
issue-unreadable-field = onleesbaar deckveld { $field }, hersteld
op-invalid-chart = Grafiek { $subject }: { $reason }
op-invalid-table = Tabel { $subject }: { $reason }

replica-corrupted = De geschiedenis van de presentatie in dit bestand is beschadigd
replica-too-large = De geschiedenis van de presentatie in dit bestand is te groot om te openen
replica-write = De geschiedenis van de presentatie kon niet worden bijgewerkt
replica-unrelated = Deze twee presentaties hebben geen gemeenschappelijke oorsprong
replica-unsupported-version = Deze presentatie gebruikt een deckformaat dat deze versie van Oku Slides niet kan lezen

widget-unreadable = De widgetdeclaraties konden niet worden gelezen
widget-fault-empty-name = Widget { $widget }: een widget of parameter heeft geen naam
widget-fault-duplicate-widget = Widget { $widget } is twee keer gedeclareerd
widget-fault-duplicate-param = Widget { $widget } declareert dezelfde parameter twee keer
widget-fault-empty-choice = Widget { $widget }: een keuze heeft geen waarden
widget-fault-default-not-in-choice = Widget { $widget }: de standaardwaarde van een keuze maakt geen deel uit van de waarden
widget-fault-default-wrong-type = Widget { $widget }: een standaardwaarde komt niet overeen met het type van zijn parameter
widget-fault-bind-not-plain = Widget { $widget }: alleen platte tekstparameters kunnen aan de presentatie worden gekoppeld
widget-fault-unknown-slot-kind = Widget { $widget }: een slot accepteert een onbekende elementwidget
widget-fault-empty-list = Widget { $widget }: een lijst heeft minstens één kolom nodig
widget-fault-column-type = Widget { $widget }: een lijstkolom kan plain, rich, number, flag, choice of asset zijn; gebruik voor vrije inhoud per item p.slot met een kind-widget
widget-fault-reserved-name = Widget { $widget }: gereserveerde naam: id, kind of een naam met /
widget-fault-list-bounds = Widget { $widget }: het maximum van een lijst ligt onder het minimum
widget-fault-list-default-too-large = Widget { $widget }: de standaardwaarde van een lijst overschrijdt de groottegrenzen van lijsten
widget-incompatible-override = Widget { $widget } vervangt de ingebouwde widget maar wijzigt of verwijdert de parameter { $param }
widget-incompatible-category = Widget { $widget } vervangt de ingebouwde widget met een andere categorie
issue-empty-item-id = lijst { $param } heeft een item zonder id
issue-duplicate-item-id = lijst { $param } heeft een dubbel item-id
issue-list-bounds = lijst { $param } heeft te weinig of te veel items
