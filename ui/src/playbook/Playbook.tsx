// The dev gallery: every playbook example rendered live, by group, with an action log.

import { Component, useRef, useState, type ErrorInfo, type ReactNode } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { SurfaceView } from "../a2ui/SurfaceView";
import { applyDefaultScheme } from "../appearance";
import { Button, Field } from "../primitives";
import { currentScheme, type Scheme } from "../scheme";
import { byGroup, componentsOf, visible, type PlaybookEntry } from "./playbookEntries";

/** The most recent actions kept in the log. */
const LOG_SIZE = 50;

/** One logged action, with a stable key. */
interface LogLine {
  id: number;
  text: string;
}

/** Keeps one example's crash inside its card instead of unmounting the gallery. */
class ExampleBoundary extends Component<{ children: ReactNode }, { error: string | null }> {
  state = { error: null as string | null };

  static getDerivedStateFromError(e: unknown): { error: string } {
    return { error: e instanceof Error ? e.message : String(e) };
  }

  componentDidCatch(e: unknown, info: ErrorInfo): void {
    console.error("playbook example", e, info.componentStack);
  }

  render(): ReactNode {
    return this.state.error ? (
      <p role="alert" className="playbook-error">
        {this.state.error}
      </p>
    ) : (
      this.props.children
    );
  }
}

/** One example: its name, note, live surface, error and JSON. */
function Example({ entry, onAction }: { entry: PlaybookEntry; onAction: (a: A2uiClientAction) => void }): ReactNode {
  const [error, setError] = useState<string | null>(null);
  return (
    <article className="playbook-example" data-example={`${entry.component}/${entry.variant}`}>
      <header>
        <h3>{entry.component}</h3>
        <span className="playbook-variant">{entry.variant}</span>
        {entry.agent && <span className="playbook-agent">agent</span>}
      </header>
      <p className="playbook-note">{entry.note}</p>
      <div className="playbook-surface">
        <ExampleBoundary>
          <SurfaceView messages={entry.messages} surfaceId={entry.surface} onAction={onAction} onError={setError} />
        </ExampleBoundary>
      </div>
      {error && (
        <p role="alert" className="playbook-error">
          {error}
        </p>
      )}
      <details>
        <summary>JSON</summary>
        <pre>{JSON.stringify(componentsOf(entry), null, 2)}</pre>
      </details>
    </article>
  );
}

/** The gallery of `entries`, filtered by component name. */
export function Playbook({ entries }: { entries: PlaybookEntry[] }): ReactNode {
  const [filter, setFilter] = useState("");
  const [scheme, setScheme] = useState<Scheme>(currentScheme);
  const [log, setLog] = useState<LogLine[]>([]);
  const nextId = useRef(0);
  const toggleScheme = (): void => {
    const next: Scheme = scheme === "dark" ? "light" : "dark";
    applyDefaultScheme(next);
    setScheme(next);
  };
  const logFrom = (entry: PlaybookEntry) => (a: A2uiClientAction): void => {
    const line = { id: nextId.current++, text: `${entry.component}/${entry.variant} ${JSON.stringify(a)}` };
    setLog((l) => [line, ...l].slice(0, LOG_SIZE));
  };
  return (
    <main className="playbook">
      <header className="playbook-bar">
        <h1>A2UI playbook</h1>
        <Field variant="menu" label="Filter">
          <input aria-label="Filter components" value={filter} onChange={(e) => setFilter(e.target.value)} />
        </Field>
        <Button onClick={toggleScheme}>{scheme === "dark" ? "Light" : "Dark"}</Button>
      </header>
      <ol className="playbook-log" aria-label="Actions">
        {log.map((line) => (
          <li key={line.id}>{line.text}</li>
        ))}
      </ol>
      {byGroup(visible(entries, filter)).map(({ group, title, entries: shown }) => (
        <section key={group} className="playbook-group">
          <h2>{title}</h2>
          {shown.map((e) => (
            <Example key={e.surface} entry={e} onAction={logFrom(e)} />
          ))}
        </section>
      ))}
    </main>
  );
}
