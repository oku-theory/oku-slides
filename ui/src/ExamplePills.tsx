/**
 * Pastilles des cas d'exemple d'un widget, au-dessus de son aperçu : un clic
 * choisit le cas montré (aperçu et réglages), « + Cas » ajoute une copie du
 * cas courant, un double-clic le renomme, la corbeille le supprime après une
 * confirmation dans la page. Les cas vivent dans le pack ; chaque écriture
 * passe par `run` (refus dit en toast, snapshot à jour). Le cas courant est
 * gardé par l'hôte (`UiState.case`).
 */
import { useState } from "react";
import type { Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";

/** Nom du cas implicite d'un widget sans cas enregistrés (`EXAMPLE_DEFAULT` côté session). */
export const EXAMPLE_DEFAULT = "Exemple";
/** Cas d'exemple d'un widget, au plus (`EXAMPLES_MAX` côté session). */
export const EXAMPLES_MAX = 12;

/** Le cas montré : le cas gardé s'il existe encore, sinon le premier. */
export function currentCase(names: string[], kept: string | null | undefined): string {
  return kept && names.includes(kept) ? kept : (names[0] ?? EXAMPLE_DEFAULT);
}

/** Un nom de nouveau cas libre : « Cas N », N à partir du nombre de cas + 1. */
export function newCaseName(names: string[]): string {
  for (let n = names.length + 1; ; n++) {
    const name = t("ui-examples-new-name", { n });
    if (!names.includes(name)) return name;
  }
}

/**
 * Ajoute au widget `widget` un cas au nom libre, copie du cas `from` ; rend
 * son nom, ou `null` si l'écriture est refusée (`run` le dit en toast).
 */
export async function addCase(
  api: TabApi,
  widget: string,
  names: string[],
  from: string,
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>,
): Promise<string | null> {
  const name = newCaseName(names);
  return (await run(api.addExample(widget, name, from))) ? name : null;
}

export interface ExamplePillsProps {
  api: TabApi;
  /** Fichier du widget. */
  widget: string;
  /** Noms des cas, dans l'ordre. */
  names: string[];
  /** Cas montré, parmi `names`. */
  current: string;
  /** Choisit le cas montré. */
  onSelect: (name: string) => void;
  /** Envoie une écriture : refus dit en toast (`null`), snapshot mis à jour. */
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
}

export default function ExamplePills({ api, widget, names, current, onSelect, run }: ExamplePillsProps) {
  const [naming, setNaming] = useState<{ from: string; value: string } | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);

  const add = () => void addCase(api, widget, names, current, run).then((name) => name && onSelect(name));
  const commit = () => {
    if (!naming) return;
    const { from } = naming;
    const to = naming.value.trim();
    setNaming(null);
    if (to === from) return;
    // Nom vide ou pris : la session le refuse, `run` le dit en toast.
    void run(api.renameExample(widget, from, to)).then((s) => s && from === current && onSelect(to));
  };
  const remove = (name: string) => {
    setRemoving(null);
    void run(api.removeExample(widget, name)).then((s) => {
      if (s && name === current) onSelect(s.examples?.[widget]?.[0] ?? names.find((n) => n !== name) ?? EXAMPLE_DEFAULT);
    });
  };

  return (
    <div className="example-pills">
      <div className="example-pills-row" role="group" aria-label={t("ui-examples")}>
        {names.map((name) =>
          naming?.from === name ? (
            <form key={name} className="example-name" onSubmit={(e) => { e.preventDefault(); commit(); }}>
              <input
                className="select"
                autoFocus
                aria-label={t("ui-examples-name")}
                value={naming.value}
                maxLength={40}
                onChange={(e) => setNaming({ from: name, value: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    commit();
                  } else if (e.key === "Escape") {
                    e.stopPropagation();
                    setNaming(null);
                  }
                }}
              />
            </form>
          ) : (
            <span key={name} className="example-pill-wrap">
              <button
                className={`chip example-pill${name === current ? " on" : ""}`}
                aria-pressed={name === current}
                title={t("ui-examples-rename-hint")}
                onClick={() => onSelect(name)}
                onDoubleClick={() => setNaming({ from: name, value: name })}
              >
                {name}
              </button>
              {names.length > 1 && (
                <button
                  className="token ghost example-remove"
                  aria-label={t("ui-examples-delete-named", { name })}
                  title={t("ui-examples-delete-named", { name })}
                  onClick={() => setRemoving(name)}
                >
                  {Icon.close}
                </button>
              )}
            </span>
          ),
        )}
        <button
          className="chip example-add"
          aria-label={t("ui-examples-add-hint")}
          title={t("ui-examples-add-hint")}
          disabled={names.length >= EXAMPLES_MAX}
          onClick={add}
        >
          {`+ ${t("ui-examples-add")}`}
        </button>
      </div>
      {removing !== null && (
        <div
          className="example-confirm"
          role="alertdialog"
          aria-label={t("ui-examples-confirm-delete", { name: removing })}
          onKeyDown={(e) => {
            if (e.key !== "Escape") return;
            e.stopPropagation();
            setRemoving(null);
          }}
        >
          <span>{t("ui-examples-confirm-delete", { name: removing })}</span>
          <button className="btn example-confirm-ok" onClick={() => remove(removing)}>
            {t("ui-examples-delete")}
          </button>
          <button className="btn example-confirm-cancel" autoFocus onClick={() => setRemoving(null)}>
            {t("ui-examples-cancel")}
          </button>
        </div>
      )}
    </div>
  );
}
