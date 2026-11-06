# Import calibration corpus (t-909, t-958)

Third-party `.pptx` test decks under Apache-2.0 or MIT, downloaded unchanged
on 2026-11-06 from the upstream repositories below, with the user's approval
of these sources. Each upstream licence (and NOTICE, for Apache projects) is
in `licenses/`. Git LFS stores the decks.

They calibrate the hi-fi import's visual gap threshold and the witness limits
(`score_threshold`, `WitnessLimit::DefaultFont`) on varied input: SmartArt,
charts, tables, themes and masters, groups, pictures, Japanese text and Strict
OOXML. They are not snapshot fixtures: tests that snapshot `SourceDeck` or
`ImportPlan` keep using the hand-written `minimal/` and `kitchen/`. Treat every
file here as untrusted bytes.

| File | Upstream (repository, branch, path) | Licence | sha256 |
|---|---|---|---|
| `poi-SmartArt.pptx` | apache/poi trunk test-data/slideshow/SmartArt.pptx | Apache-2.0 (`licenses/apache-poi-LICENSE.txt`) | `b97e4c6d2ee1dd4094f50f9043820610268dd452d7717c537f5456636adcc353` |
| `poi-smartart-simple.pptx` | apache/poi trunk test-data/slideshow/smartart-simple.pptx | Apache-2.0 (`licenses/apache-poi-LICENSE.txt`) | `238ff6433dac00b2722edb9a29d97f56ab4f3c9c37f33a73d6f53baf494916aa` |
| `poi-table-with-no-theme.pptx` | apache/poi trunk test-data/slideshow/table-with-no-theme.pptx | Apache-2.0 (`licenses/apache-poi-LICENSE.txt`) | `d6b3cca13b8bdeaa66df184883ace4fcbc6bf7a88a9547abdbd01875caf5af4e` |
| `poi-table-with-theme.pptx` | apache/poi trunk test-data/slideshow/table-with-theme.pptx | Apache-2.0 (`licenses/apache-poi-LICENSE.txt`) | `571d0cd2e1e9767249b3a02544c7771c883c48bb191264409729c7cec574ab48` |
| `poi-themes.pptx` | apache/poi trunk test-data/slideshow/themes.pptx | Apache-2.0 (`licenses/apache-poi-LICENSE.txt`) | `3ea86e4ee84a8bca6b99fe3a9f2a9a9186c0655df28ff92a2d02da24926c64ba` |
| `poi-with_japanese.pptx` | apache/poi trunk test-data/slideshow/with_japanese.pptx | Apache-2.0 (`licenses/apache-poi-LICENSE.txt`) | `f0b1efbfbd70f7ee2395dc2505639f99449971ba404a6ec8cf1b97d82cf45c10` |
| `powertools-AnglesTheme.pptx` | OpenXmlDev/Open-Xml-PowerTools vNext NewPptxPresentations/AnglesTheme.pptx | MIT (`licenses/open-xml-powertools-LICENSE.txt`) | `9c54175a9c8d23cc17fff7f43262c8cd7a416ff57ead82a8638113c9de34f383` |
| `powertools-ApexTheme.pptx` | OpenXmlDev/Open-Xml-PowerTools vNext NewPptxPresentations/ApexTheme.pptx | MIT (`licenses/open-xml-powertools-LICENSE.txt`) | `d78b4da2b53e51ad9400f5c534ef95716af8e5578bf92f3b4369c8d64d84a139` |
| `powertools-WaveFormTheme.pptx` | OpenXmlDev/Open-Xml-PowerTools vNext NewPptxPresentations/WaveFormTheme.pptx | MIT (`licenses/open-xml-powertools-LICENSE.txt`) | `bf830f5bf38ec11cc232c99c606f3e85ac8733a9e88b07994ef61591c7ae548e` |
| `pythonpptx-cht-chart-type.pptx` | scanny/python-pptx master features/steps/test_files/cht-chart-type.pptx | MIT (`licenses/python-pptx-LICENSE.txt`) | `d0a7979d05970aa83fd8406f0829cd0019853a7bc4e16f8cda814586fc2eaccc` |
| `pythonpptx-shp-groupshape.pptx` | scanny/python-pptx master features/steps/test_files/shp-groupshape.pptx | MIT (`licenses/python-pptx-LICENSE.txt`) | `37c9a0bd919f9fc4e89600939cbaf798e79c6aaf96406aee5bba2828cea3e3a6` |
| `pythonpptx-shp-picture.pptx` | scanny/python-pptx master features/steps/test_files/shp-picture.pptx | MIT (`licenses/python-pptx-LICENSE.txt`) | `20b6295563e217804af81f8b5fee5e5cef9e766ac401d825d536ffe6d674edd0` |
| `sdk-Chart_2D.pptx` | dotnet/Open-XML-SDK main …/TestDataStorage/v2FxTestFiles/presentation/Chart_2D.pptx | MIT (`licenses/open-xml-sdk-LICENSE.txt`) | `dad57bde3989dbc48e5c52988773ccf74d1cf56af875e5366e4c92992ce258f3` |
| `sdk-Photo_Album3.pptx` | dotnet/Open-XML-SDK main …/TestDataStorage/O14ISOStrict/PowerPoint/Photo Album3.pptx | MIT (`licenses/open-xml-sdk-LICENSE.txt`) | `9d6fe63f940b267a601629deb7b4a3cd7a092a2c0c12d0714d49e8c752456474` |
| `sdk-SmartArt_OrgChart1.pptx` | dotnet/Open-XML-SDK main …/TestDataStorage/v2FxTestFiles/presentation/SmartArt_OrgChart1.pptx | MIT (`licenses/open-xml-sdk-LICENSE.txt`) | `534b94adefd42e0874fc4c8c45fb7b1dbcd2df63dbec20d68762553869bc57dd` |
| `sdk-SmartArt_OrgChart1_3D.pptx` | dotnet/Open-XML-SDK main …/TestDataStorage/v2FxTestFiles/presentation/SmartArt_OrgChart1_3D.pptx | MIT (`licenses/open-xml-sdk-LICENSE.txt`) | `ba05e16222bd9295aac653ca738963fcc9f885153e416c1f2ba652eeb5f109d0` |
| `sdk-Table_Large.pptx` | dotnet/Open-XML-SDK main …/TestDataStorage/v2FxTestFiles/presentation/Table_Large.pptx | MIT (`licenses/open-xml-sdk-LICENSE.txt`) | `07aa3b2665961a001e4a05e0de6348babd67080de38387e780768ac5fbb8eb06` |
| `tika-testMSChart-govdocs-428996.pptx` | apache/tika main …/tika-parser-microsoft-module/src/test/resources/test-documents/testMSChart-govdocs-428996.pptx | Apache-2.0 (`licenses/apache-tika-LICENSE.txt`) | `4eef2f71094586d84e8b37a07d9d6b6b981d0532a2d1d5a217d3734abff77b91` |
| `tika-testPPT_groups.pptx` | apache/tika main …/tika-parser-microsoft-module/src/test/resources/test-documents/testPPT_groups.pptx | Apache-2.0 (`licenses/apache-tika-LICENSE.txt`) | `e8d4cd55569be22849a584bff949d07e8c16abbd00b59feedc43f17c33b4061f` |

Paths shortened with `…`: Open-XML-SDK under
`test/DocumentFormat.OpenXml.Tests.Assets/assets/TestDataStorage/`, Tika under
`tika-parsers/tika-parsers-standard/tika-parsers-standard-modules/`.
`tika-testMSChart-govdocs-428996.pptx` comes, by its name, from the govdocs1
corpus of US government documents (public domain). The PowerTools theme decks
carry Microsoft's built-in Office themes, shipped under MIT in that repository.
