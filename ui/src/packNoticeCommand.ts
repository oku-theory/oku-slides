// Les évènements du bandeau du pack, chacun nommé comme la requête qu'il envoie.

const COMMANDS = ["pack_use_mine", "pack_make_current", "pack_notice_close"] as const;

/** Une requête du bandeau du pack. */
export type PackNoticeCommand = (typeof COMMANDS)[number];

/** La requête d'un évènement du bandeau, ou `null` s'il n'en est pas une. */
export const packNoticeCommand = (name: string): PackNoticeCommand | null =>
  COMMANDS.find((c) => c === name) ?? null;
