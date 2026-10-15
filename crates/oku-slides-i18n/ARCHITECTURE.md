# oku-slides-i18n

Oku Slides languages: every user-facing message as Fluent, compiled into the binary.
English is the reference; 20 locales ship. No dependency on any other oku-slides crate,
so every crate can use it.

## Modules

- `lib.rs`: the `locales!` macro list (code and English name per locale) builds `LOCALES`
  and `ENGLISH_NAMES`; `locale!` embeds one locale's `.ftl` files with `include_str!`.
  - `Localizer`: one locale's `FluentBundle`, with an English `Localizer` as fallback for
    a missing message; `tr`, `tr_with`, `try_tr`, `has`.
  - `Localize`: trait for errors and reports shown to the user; `Display` stays English.
  - `resolve`, `shipped`, `available`, `english_name`, `english`, `REFERENCE`.
  - `ui_messages`: the raw `ui.ftl` sources (English, then the chosen locale) handed to
    the React UI's own Fluent engine.
- `message.rs`: `Message` (Fluent id plus `Arg` values), built without a locale inside
  an error and spoken later in the user's language or in English.
- `keys.rs`: `platform_keys` and `pc_keys` rewrite macOS shortcuts (`⌘⇧S`) as
  `Ctrl+Shift+S` off macOS.

## Locale choice

`Localizer::new(requested)` → `resolve`: the requested locale if shipped, else the system
locale (`sys-locale`), else `en`. `shipped` maps regional tags (`fr-CA` → `fr`) and
Chinese regions to `zh-Hans` or `zh-Hant`. Bidi isolation marks are off so paths and
numbers print as is in a terminal.

## On-disk layout

`src/locales/<locale>/` holds one file per domain: `core.ftl`, `document.ftl`,
`session.ftl`, `stt.ftl`, `cli.ftl`, `chart.ftl`, `agent.ftl`, then `ui.ftl` last (the
React UI). Keys are prefixed by domain (`cli-…`, `chart-…`, `import-…`).

## Outside Fluent

The agent's instructions (`oku-slides-agent/src/instructions/*.md`) are prompts, not
user-facing text: English only, never translated; each turn passes the interface language.

## Extension points

- A locale: one line in `locales!` and a `src/locales/<locale>/` folder with every file.
- A domain: a new `.ftl` file in every locale and one `include_str!` line in `locale!`.

## Dependencies

No oku-slides crate. External: `fluent-bundle`, `unic-langid`, `sys-locale`.
