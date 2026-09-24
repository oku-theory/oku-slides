import { useCallback, useEffect, useRef, useState } from "react";
import type { SttModel, SttStatus } from "./api";
import { stt } from "./api";

const LANGUAGES: [string | null, string][] = [
  [null, "Détection automatique"],
  ["fr", "Français"],
  ["en", "English"],
  ["de", "Deutsch"],
  ["es", "Español"],
  ["it", "Italiano"],
  ["pt", "Português"],
  ["nl", "Nederlands"],
];

const mo = (bytes: number) =>
  bytes >= 1e9 ? `${(bytes / 1e9).toLocaleString("fr-FR", { maximumFractionDigits: 1 })} Go` : `${Math.round(bytes / 1e6)} Mo`;

/** Fin du texte provisoire : c'est là qu'on parle. */
const tail = (t: string, n = 150) => (t.length > n ? `…${t.slice(t.length - n).replace(/^\S*\s/, "")}` : t);

const clock = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;

const MicIcon = (
  <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="3" width="6" height="11" rx="3" /><path d="M5 11a7 7 0 0 0 14 0" /><path d="M12 18v3" /></svg>
);
const StopIcon = <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2.5" /></svg>;
const GearIcon = (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></svg>
);

interface Props {
  /** Où ira le texte, affiché au repos (« dans le titre », « nouveau bloc »…). */
  destination: string;
  /** Mots utiles à Whisper : titres, noms propres du deck. */
  prompt: string;
  /** Texte final d'une dictée. */
  onText: (text: string) => void;
  onError: (message: string) => void;
}

/** L'appareil de dictée : micro, niveau, texte provisoire, réglages du modèle. */
export default function Dictation({ destination, prompt, onText, onError }: Props) {
  const [status, setStatus] = useState<SttStatus | null>(null);
  const [settings, setSettings] = useState(false);
  const [levels, setLevels] = useState<number[]>(() => Array(28).fill(0));
  const lastSeq = useRef<number | null>(null);
  const lastError = useRef<string | null>(null);
  const handlers = useRef({ onText, onError });
  handlers.current = { onText, onError };

  const receive = useCallback((s: SttStatus) => {
    setStatus(s);
    if (lastSeq.current === null) lastSeq.current = s.seq;
    else if (s.seq !== lastSeq.current) {
      lastSeq.current = s.seq;
      if (s.text) handlers.current.onText(s.text);
      else handlers.current.onError("Rien entendu : parlez un peu plus près du micro");
    }
    if (s.error && s.error !== lastError.current) handlers.current.onError(s.error);
    lastError.current = s.error;
    if (s.state === "recording") setLevels((l) => [...l.slice(1), s.level]);
  }, []);

  const send = useCallback(
    (p: Promise<SttStatus>) => p.then(receive).catch((e) => handlers.current.onError(String(e instanceof Error ? e.message : e))),
    [receive],
  );

  useEffect(() => {
    send(stt.status());
  }, [send]);

  // Sondage tant que quelque chose se passe (téléchargement, écoute, transcription).
  const busy = status && status.state !== "idle";
  useEffect(() => {
    if (!busy) return;
    const t = window.setInterval(() => send(stt.status()), status?.state === "downloading" ? 400 : 90);
    return () => window.clearInterval(t);
  }, [busy, status?.state, send]);

  useEffect(() => {
    if (status?.state === "idle") setLevels((l) => l.map(() => 0));
  }, [status?.state]);

  const toggle = useCallback(() => {
    if (!status) return;
    if (status.state === "idle") {
      setSettings(false);
      send(stt.start(prompt || null));
    } else if (status.state === "recording") send(stt.stop());
  }, [status, prompt, send]);

  // ⌘D (ou Ctrl+D) partout, même pendant la saisie d'un texte.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === "d") {
        e.preventDefault();
        e.stopPropagation();
        toggle();
      } else if (e.key === "Escape" && status && (status.state === "recording" || status.state === "downloading")) {
        e.preventDefault();
        e.stopPropagation();
        send(stt.cancel());
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [toggle, status, send]);

  if (!status) {
    return (
      <div className="device object" aria-label="Dictée">
        <button className="mic" disabled aria-label="Dicter">{MicIcon}</button>
        <div className="spacer caption">Préparation de la dictée…</div>
      </div>
    );
  }

  const model = status.models.find((m) => m.id === status.config.model);
  const language = LANGUAGES.find(([code]) => code === status.config.language)?.[1] ?? status.config.language;
  const { state } = status;

  let body: React.ReactNode;
  if (state === "downloading" && status.download) {
    const d = status.download;
    const pct = Math.min(100, Math.floor((d.received / Math.max(1, d.total)) * 100));
    const name = status.models.find((m) => m.id === d.model)?.label.split(" (")[0];
    body = (
      <div className="dictation-body">
        <div className="dictation-line">
          <span className="dictation-state">Téléchargement du modèle {name}</span>
          <span className="caption">{pct} % · {mo(d.received)} sur {mo(d.total)} · une seule fois</span>
        </div>
        <div className="progress" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
          <span style={{ width: `${pct}%` }} />
        </div>
      </div>
    );
  } else if (state === "recording" || state === "transcribing") {
    body = (
      <div className="dictation-body">
        <div className="dictation-line">
          <span className="dictation-state">
            {state === "recording" ? <><span className="rec-dot" />J'écoute · {clock(status.seconds)}</> : "Transcription…"}
          </span>
          {state === "recording" && (
            <span className="wave" aria-hidden>
              {levels.map((l, i) => <span key={i} style={{ transform: `scaleY(${Math.max(0.08, Math.min(1, l * 1.6))})` }} />)}
            </span>
          )}
        </div>
        <div className={`partial${status.partial ? "" : " empty"}`} aria-live="polite">
          {tail(status.partial) || (state === "recording" ? "Parlez, le texte apparaît ici…" : "")}
        </div>
      </div>
    );
  } else {
    body = (
      <div className="dictation-body">
        <div className="dictation-line">
          <span className="dictation-state">Dicter <span className="caption">→ {destination}</span></span>
          <span className="caption">
            {model?.label.split(" (")[0]} · {language}
            {model && !model.installed ? ` · ${mo(model.size)} à télécharger` : ""}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className={`device object dictation ${state}`} aria-label="Dictée">
      <button
        className={`mic${state === "recording" ? " live" : ""}`}
        aria-label={state === "recording" ? "Terminer la dictée" : "Dicter"}
        title={state === "recording" ? "Terminer (⌘D)" : "Dicter (⌘D)"}
        disabled={state === "downloading" || state === "transcribing"}
        // Ne pas voler le focus : un texte en cours d'édition reste ouvert.
        onMouseDown={(e) => e.preventDefault()}
        onClick={toggle}
      >
        {state === "recording" ? StopIcon : MicIcon}
        {state === "recording" && <span className="mic-ring" style={{ transform: `scale(${1 + status.level * 0.5})` }} />}
      </button>
      {body}
      {state === "recording" || state === "downloading" ? (
        <button className="btn ghost" onMouseDown={(e) => e.preventDefault()} onClick={() => send(stt.cancel())}>Annuler</button>
      ) : (
        <button
          className="token ghost"
          aria-label="Réglages de la dictée"
          aria-expanded={settings}
          disabled={state !== "idle"}
          onClick={() => setSettings((v) => !v)}
        >
          {GearIcon}
        </button>
      )}

      {settings && state === "idle" && (
        <div className="dictation-settings object" role="dialog" aria-label="Réglages de la dictée">
          <div className="settings-title">Modèle de reconnaissance</div>
          <p className="caption">Tout reste sur cet ordinateur. Tous les modèles comprennent plusieurs langues.</p>
          <div className="model-list" role="radiogroup">
            {status.models.map((m) => {
              const [name, detail] = m.label.replace(")", "").split(" (");
              return (
                <label key={m.id} className={`model${m.id === status.config.model ? " chosen" : ""}`}>
                  <input
                    type="radio"
                    name="stt-model"
                    checked={m.id === status.config.model}
                    onChange={() => send(stt.configure(m.id as SttModel, status.config.language))}
                  />
                  <span className="model-name">{name}</span>
                  <span className="caption">{MODEL_NOTES[m.id]} · {detail?.replace("multilingue, ", "")}</span>
                  <span className={`badge${m.installed ? " ok" : ""}`}>{m.installed ? "installé" : "à télécharger"}</span>
                </label>
              );
            })}
          </div>
          <label className="settings-row">
            <span className="caption">Langue parlée</span>
            <select
              className="select"
              value={status.config.language ?? ""}
              onChange={(e) => send(stt.configure(status.config.model, e.target.value || null))}
            >
              {LANGUAGES.map(([code, name]) => <option key={code ?? "auto"} value={code ?? ""}>{name}</option>)}
            </select>
          </label>
          {model && !model.installed && (
            <button className="btn" onClick={() => send(stt.download(model.id))}>Télécharger maintenant ({mo(model.size)})</button>
          )}
        </div>
      )}
    </div>
  );
}

const MODEL_NOTES: Record<string, string> = {
  turbo: "Recommandé, rapide et précis",
  large: "Le plus précis, plus lent",
  base: "Léger, pour les petites machines",
};
