// The knowledge tab's tree: every file by directory, the open one marked, awaiting ones badged.
//
// A directory with an `index.md` is a row opening that page; one without is a plain name. An
// awaiting file shows its file name without bidi or format characters, never a title.

import { t } from "../i18n";
import { Button } from "../primitives";
import { nodeLabel, plainPath, type TreeDir } from "./knowledgeTree";
import type { KnowledgeNode } from "./types";

interface Props {
  root: TreeDir;
  /** The file of the shown sub-tab. */
  active: string | null;
  onOpen(node: KnowledgeNode): void;
}

/** One file's row; `label` overrides its own (a directory's page shows the directory). */
function Row({ node, label, active, onOpen }: { node: KnowledgeNode; label?: string; active: string | null; onOpen(node: KnowledgeNode): void }) {
  return (
    <Button
      ghost className="knowledge-tree-row" onClick={() => onOpen(node)}
      aria-current={node.id === active ? "page" : undefined} data-awaiting={node.awaiting || undefined}
    >
      <bdi className="knowledge-tree-name">{label ?? nodeLabel(node)}</bdi>
      {node.awaiting && <span className="knowledge-awaiting-badge">{t("ui-knowledge-awaiting-badge")}</span>}
    </Button>
  );
}

/** A directory's sub-directories then files, each in its own list item. */
function Entries({ dir, active, onOpen }: { dir: TreeDir; active: string | null; onOpen(node: KnowledgeNode): void }) {
  return (
    <>
      {dir.dirs.map((sub) => (
        <li key={sub.path} className="knowledge-tree-dir">
          {sub.index ? (
            <Row node={sub.index} label={sub.index.title ?? plainPath(sub.name)} active={active} onOpen={onOpen} />
          ) : (
            <bdi className="knowledge-tree-name">{plainPath(sub.name)}</bdi>
          )}
          <ul>
            <Entries dir={sub} active={active} onOpen={onOpen} />
          </ul>
        </li>
      ))}
      {dir.files.map((file) => (
        <li key={file.id}>
          <Row node={file} active={active} onOpen={onOpen} />
        </li>
      ))}
    </>
  );
}

/** The whole tree, the root's own page first. */
export function KnowledgeTreeView({ root, active, onOpen }: Props) {
  return (
    <nav className="knowledge-tree" aria-label={t("ui-knowledge-tree-label")}>
      <ul>
        {root.index && (
          <li>
            <Row node={root.index} active={active} onOpen={onOpen} />
          </li>
        )}
        <Entries dir={root} active={active} onOpen={onOpen} />
      </ul>
    </nav>
  );
}
