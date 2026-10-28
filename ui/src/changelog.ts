// The releases of CHANGELOG.md, read for the About dialog. The file is written by git-cliff from
// cliff.toml: `## [version] - date`, `### Group`, `- *(scope)* [**breaking**] text`.

/** One change of a release. */
export interface ChangeItem {
  /** Scope in parentheses, e.g. `session,ui`; `null` without one. */
  scope: string | null;
  /** Marked `[**breaking**]`. */
  breaking: boolean;
  /** The commit description, as written. */
  text: string;
}

/** A group of changes (Added, Fixed…). */
export interface ChangeGroup {
  title: string;
  items: ChangeItem[];
}

/** One release, newest first in the file. */
export interface Release {
  version: string;
  date: string;
  groups: ChangeGroup[];
}

const RELEASE = /^## \[([^\]]+)\](?: - (\S+))?/;
const GROUP = /^### (.+)/;
const ITEM = /^- (?:\*\(([^)]*)\)\* )?(\[\*\*breaking\*\*\] )?(.*)/;

/** Parses the releases of a changelog in git-cliff's format, in file order. */
export function parseChangelog(source: string): Release[] {
  const releases: Release[] = [];
  for (const line of source.split("\n")) {
    const release = RELEASE.exec(line);
    if (release) {
      releases.push({ version: release[1], date: release[2] ?? "", groups: [] });
      continue;
    }
    const current = releases[releases.length - 1];
    if (!current) continue;
    const group = GROUP.exec(line);
    if (group) {
      current.groups.push({ title: group[1].trim(), items: [] });
      continue;
    }
    const item = ITEM.exec(line);
    const into = current.groups[current.groups.length - 1];
    if (item && into) into.items.push({ scope: item[1] ?? null, breaking: !!item[2], text: item[3].trim() });
  }
  return releases;
}
