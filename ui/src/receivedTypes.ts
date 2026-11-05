// Agent guides of packs and themes, and the received versions a deck brings (t-1414).

/** The two agent guides of a pack or theme, as written in the session's version. */
export interface Guides {
  edit?: string;
  use?: string;
}

/** Where a version of a pack or theme came from. */
export type GuideOrigin = "builtin" | "local" | "received";

/** The guides of the edited pack or theme, with the length one may reach. */
export interface GuidesView {
  guides: Guides;
  max_chars: number;
  /** Id of the edited pack or theme. */
  id: string;
  /** The edited version. */
  version: string;
  /** Where that version came from. */
  origin: GuideOrigin;
  /** The user approved this version's guides. */
  approved: boolean;
  /** Approve may be offered: the version awaits, and `guides` are the ones it would approve. */
  approvable: boolean;
  /** A theme draft's guide kept from an awaiting base, beside one written here: no Approve. */
  kept_awaiting: boolean;
}

/** One version of a pack or theme, by id and digest. */
export interface VersionRef {
  id: string;
  version: string;
}

/** What a pack or theme version holds, read from its files alone. */
export interface Capabilities {
  kind: "widgets" | "layouts" | "theme";
  /** Name from the manifest, cleaned for display. */
  name: string;
  namespace: string;
  /** One-sentence description; empty if absent. */
  description: string;
  /** Widget or layout names, `_commun` excluded; empty for a theme. */
  items: string[];
  /** Namespaces a pack requires, or the themes a theme extends. */
  requires: string[];
  /** Packs: component token paths; themes: token group names. */
  tokens: string[];
  /** Font families named in the tokens. */
  fonts: string[];
  /** Number of layout profiles the import can pick. */
  profiles: number;
  /** Total bytes of the Typst sources (0 for a theme). */
  code_bytes: number;
}

/** Full lengths of the capability lists, which hold at most 50 names each. */
export interface CapabilityTotals {
  items: number;
  requires: number;
  tokens: number;
  fonts: number;
}

/** How a received version relates to the user's library. */
export type ReceivedRelation = "new_pack" | "new_version" | "held";

/** A version the open deck brings, to validate before it enters the library. */
export interface ReceivedReview {
  id: string;
  version: string;
  capabilities: Capabilities;
  /** How many names each capability list had before the cap. */
  totals: CapabilityTotals;
  relation: ReceivedRelation;
  /** True when `acceptReceived` adds it to the library. */
  install: boolean;
  /** Why it stays in the deck, as a full localized sentence; then `install` is false. */
  blocker: string | null;
  /** Its guides as the agent would read them; empty when none await approval. */
  guides: Guides;
}

/** Which of the two guides. */
export type GuideKind = "edit" | "use";
