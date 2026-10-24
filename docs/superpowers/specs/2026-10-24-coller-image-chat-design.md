# Pasting an image into the agent chat — Design

## Goal

Let the user paste an image into the agent draft. The agent **sees** it (an ACP image
block, when it can receive images) and can **add it to the deck** by placing a
`base/image` element that references it. A pasted image enters the deck only when an op
places it.

## Decisions

- **Attachment dropped at paste time (approach C).** Pasting sends the bytes to the host
  at once (`paste_image` command); the host validates and stores them per tab, in memory
  (the draft itself is not persisted), and the draft holds only their refs. Rejected: sending the bytes with the prompt (lost by the detached
  window and by resume, late errors) and adding every image to the deck on send (orphan
  captures in the sources zip, outside the turn's undo).
- **Id = asset id.** An attachment's id is `asset_id(bytes)` (`sha256:<hex>`): the same
  id is cited in the prompt, passed to `apply` and stored in the bundle. Pasting the same
  image twice stores it once.
- **Formats:** PNG, JPEG, GIF, WebP only (rendered by Typst as is). SVG, BMP, TIFF, EMF
  and the rest are refused with a localized error; no resizing (no `image` dependency).
- **Limits (`[paste]` in `config.default.toml`):** `max_image_mib` 10 per image,
  `max_image_pixels` 1 << 26 (the import's value), `max_draft_images` 8 per draft,
  `max_kept_mib` 100 kept per tab, `prompt_image_mib` 5, the largest image sent to the
  agent as an image block (measured base64-encoded, the size vision APIs check),
  `prompt_image_max_side` 8000 (longest side, pixels), `prompt_total_mib` 20 (encoded
  total of the image blocks of one prompt; the images beyond it are listed only).
- **Text first on paste:** a clipboard with non-empty `text/plain` pastes its text only;
  otherwise its `image/*` files become attachments.
- **Text required:** a prompt needs text; images alone do not send.
- **Unknown asset ids are refused:** an asset id that the new deck cites, the old one did
  not, and neither the bundle nor the tab's attachments hold fails with
  `session-asset-unknown`. Decks that already lack an asset (`roadmap.json` cites
  `sha256:0000`) still open and edit; `validate` keeps reporting them.
- **No bytes in the thread:** the user bubble shows one chip per image (type and size),
  never a thumbnail; the saved thread stays within `THREAD_BYTES`.

## Components

- **`oku_slides_core::media`** (new, public): `sniff` and `dimensions` move here from
  `oku-slides-import` (`pptx/media.rs`) with their tests; the import calls them from core.
- **`Attachments`** (`oku-slides-session`, per tab session, in memory): id → `{bytes, mime,
  width, height}`. `Session::paste_image(bytes, keep)` validates (sniffed type, byte cap,
  header dimensions, pixel cap) and returns an `AttachmentRef {id, mime, width, height}`.
  Past `max_kept_mib`, the oldest images not in `keep` (the draft's ids) are forgotten; a
  forgotten id is then refused like any unknown one.
- **`AgentCommand::PasteImage { tab, data }`** (`data` base64) and **`RemoveImage { tab,
  id }`**, on the Tauri `agent` command and `POST /api/agent` (behind `guard::check`,
  within `MAX_BODY`). `PasteImage` refuses past `max_draft_images`, stores the image in
  the tab session and adds its ref to the draft; refusals are localized `CommandError`s.
- **Draft:** `TabLink`'s draft becomes `{text, images: Vec<AttachmentRef>}`. The host is
  the authority for images: every `draft` event carries them (each view adopts them, its
  own echoes included), `set_draft` still carries text only, and closing the draft drops
  its images.
- **`AgentCommand::Prompt`** gains `images: Vec<String>`. The host resolves each id in the
  tab's attachments; an unknown id is a `CommandError` and nothing is sent. `Instruction`
  gains `images: Vec<AttachmentRef>`; bytes are read from the store when the prompt is
  built. A shortcut word (`matches_shortcut`) is recognized only without images.
- **`SessionCaps.image`**, read from `agent_capabilities.prompt_capabilities.image` in
  `link::handshake`.
- **Prompt blocks** (`turn.rs`), the prompt text staying last:
  1. first-turn instructions, import instructions, context (unchanged);
  2. a "Pasted images" text block: per image its number, id, type and size, and how to
     place it (`apply` → `add_element {kind: "base/image", params: {asset: "<id>"}}`, or
     `set_param asset`); an image not sent as a block says why (agent receives no images,
     larger than the prompt limits: encoded size, side, or the per-prompt total);
  3. one `ContentBlock::Image(ImageContent::new(base64, mime))` per image when
     `caps.image` and the image is within the prompt limits;
  4. the prompt text.
- **`instructions/agent.md`:** pasted images are the user's; place them by their id;
  never write an asset id that was not given.
- **`Session::apply`:** a new step `embed_op_assets(&ops)` after `embed_op_themes`. For
  each value of a `ParamType::Asset` parameter set by an op (`add_element`, `set_param`,
  `set_kind` params; the type read from the widget registry): in the bundle, nothing; in
  the attachments, `add_asset(bytes)`; in neither, `asset-unknown`. The asset enters
  before rendering and stays even if rendering fails or the turn is undone (like embedded
  packs and imported assets), so redo always finds its bytes.
- **Thread bubble:** `ComponentKind::Prompt { text, images: Vec<PromptImage> }` (mime,
  width, height; omitted when empty) and `turnComponents.tsx` (zod `images` optional,
  still `strict()`) change in the same commit. `AgentEvent::UserText` and `TurnView`
  carry the metadata. `CATALOG_ID` stays `v4`: the field is optional, so saved threads
  read back unchanged, and the UI and the host ship together.
- **UI draft** (`DictationDraft.tsx`, new hook `usePastedImages.ts`): `onPaste` on the
  draft section (editing field and read card); `agentApi(tab).pasteImage`; a chip row under
  the text, with a thumbnail from a `blob:` URL of the local file in the pasting window
  (revoked on removal and unmount; the view computes the same `sha256:` id with
  `crypto.subtle`) and "PNG 1200×800" elsewhere; a labelled × button per chip; host
  refusals in the toast (`say`), like other draft refusals; `onSend(text, ids)`.
- **Strings:** Fluent keys in `session.ftl`, `agent.ftl` and `ui.ftl`, in all 20 locales,
  English first.

## Data flow

paste → `paste_image` → validate → `Attachments` + draft refs → draft synced to both windows →
send → `Prompt {text, images}` → `Instruction` → prompt blocks → agent → `apply`
(`add_element base/image`) → `embed_op_assets` → `add_asset` → render worker → slide SVG.

## Untrusted bytes

Pasted bytes are validated at paste time (magic bytes, header dimensions, caps), rendered
only by Typst in the bounded render worker and shown as slide SVGs. The only direct
display is the thumbnail in the pasting window, built from the user's own local file. No
route serves attachment or asset bytes.

## Known limits

- An asset placed then undone stays in the sources zip, unreferenced. No garbage
  collection: knowing that no history branch references it is not cheap with Loro.
- An image between `prompt_image_mib` and `max_image_mib` can be placed but is not
  shown to the agent; the "Pasted images" block says so.
- Pasted images do not survive an app restart, like the draft; an agent citing an id
  from before a restart gets `session-asset-unknown`.
- Out of scope: drag and drop, an "attach file" button (both would reuse `paste_image`
  and `embed_op_assets`).

## Tests

- **Rust unit** (`x_tests.rs`, speculoos, rstest tables): `core::media` (moved tests),
  `asset_refs`; `Attachments` dedup, caps, eviction outside the draft; each paste refusal
  on its key and in French; prompt block order with and without
  `caps.image` and above `prompt_image_bytes`; `Session::apply` commits a referenced
  attachment, adds nothing without a reference, refuses an unknown id.
- **UI render** (`*.render.test.tsx`): image paste calls `onPasteImages`; text paste stays text;
  text plus image pastes text only; chip, removal, error notice, send disabled without
  text; `Prompt` bubble with images; zod refuses an unknown field.
- **e2e** (`ui/e2e/260-coller-image.cjs`): open the draft (⌘K), dispatch a synthetic
  `ClipboardEvent` with a PNG fixture, check the chip, send; in `E2E_AGENT_LOG`, find a
  `session/prompt` with the "Pasted images" block naming the id and a `type: "image"`
  block (fake agent script flag `prompt_image`, in `fixtures/agent/e2e.json`). The
  fixture's id is its fixed `sha256`, so the scripted turn's `mcp` step calls `apply`
  with `add_element base/image` on that id; the spec checks the element appears on the
  slide.
