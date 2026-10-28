// The About dialog: the app version and the changes of each release (CHANGELOG.md).
// Group titles and changes stay in English, as written by git-cliff: they are data, not UI strings.

import { useRef } from "react";
import type { Release } from "./changelog";
import { t } from "./i18n";
import { useModalFrame } from "./modalFrame";
import { Button } from "./primitives";

interface Props {
  /** The running app version. */
  version: string;
  /** Releases of the changelog, newest first. */
  releases: Release[];
  /** Closes the dialog (button or Escape). */
  onClose: () => void;
}

/** About Oku Slides: version, then every release with its changes. */
export default function AboutDialog({ version, releases, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onClose });
  return (
    <div className="import-veil">
      <div
        ref={ref}
        tabIndex={-1}
        className="import-dialog about-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="about-title"
        aria-describedby="about-version"
        data-armed={armed || undefined}
        onPointerDown={(e) => e.stopPropagation()}
        onClickCapture={onClickCapture}
      >
        <h2 id="about-title" className="settings-title">{t("ui-about")}</h2>
        <p id="about-version" className="caption about-version">{t("ui-about-version", { version })}</p>
        <section className="about-changelog" aria-label={t("ui-about-changelog")} tabIndex={0}>
          {releases.map((r) => (
            <article key={r.version} className="about-release">
              <h3>{r.date ? `${r.version} · ${r.date}` : r.version}</h3>
              {r.groups.map((g) => (
                <div key={g.title}>
                  <h4 className="caption">{g.title}</h4>
                  <ul>
                    {g.items.map((item, i) => (
                      <li key={i}>
                        {item.scope && <span className="about-scope">{item.scope}</span>}
                        {item.breaking && <span className="about-breaking">{t("ui-about-breaking")}</span>}
                        {item.text}
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </article>
          ))}
        </section>
        <div className="import-actions about-actions">
          <Button primary onClick={onClose}>{t("ui-about-close")}</Button>
        </div>
      </div>
    </div>
  );
}
