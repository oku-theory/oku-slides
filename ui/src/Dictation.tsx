import { useCallback, useEffect, useRef, useState } from "react";
import type { SttConfig, SttModel, SttStatus } from "./api";
import { stt } from "./api";
import { t } from "./i18n";
import type { DictationMode } from "./agent/keys";
import { dictationShortcut } from "./agent/keys";

const SILENCES = [1, 1.5, 2, 3, 5];

const secs = (s: number) => t("ui-seconds", { value: s });

/** Langues de la dictée, chacune dans sa langue ; la détection automatique dans celle de l'interface. */
const languages = (): [string | null, string][] => [
  [null, t("ui-dictation-language-auto")],
  ["fr", "Français"],
  ["en", "English"],
  ["de", "Deutsch"],
  ["es", "Español"],
  ["it", "Italiano"],
  ["pt", "Português"],
  ["nl", "Nederlands"],
];

const mo = (bytes: number) =>
  bytes >= 1e9 ? t("ui-size-gb", { value: bytes / 1e9 }) : t("ui-size-mb", { value: Math.round(bytes / 1e6) });

/** Fin du texte : c'est là qu'on parle. */
const tail = (s: string, n = 150) => (s.length > n ? `…${s.slice(s.length - n).replace(/^\S*\s/, "")}` : s);

/** Texte définitif puis aperçu de la phrase en cours, coupés par la fin. */
function transcript(committed: string, partial: string, n = 160): [string, string] {
  const p = tail(partial, n);
  const room = Math.max(0, n - p.length);
  const c = room === 0 ? "" : tail(committed, room);
  return [c, p];
}

const clock = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;

const MicIcon = (
  <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="3" width="6" height="11" rx="3" /><path d="M5 11a7 7 0 0 0 14 0" /><path d="M12 18v3" /></svg>
);
const StopIcon = <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2.5" /></svg>;
const AgentIcon = (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3l1.8 4.7L18.5 9.5l-4.7 1.8L12 16l-1.8-4.7L5.5 9.5l4.7-1.8z" /><path d="M18 15l.8 2.2L21 18l-2.2.8L18 21l-.8-2.2L15 18l2.2-.8z" /></svg>
);
const GearIcon = (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></svg>
);

interface Props {
  /** Où ira le texte, affiché au repos (« dans le titre », « nouveau bloc »…). */
  destination: string;
  /** Mots utiles à Whisper : titres, noms propres du deck. */
  prompt: string;
  /** Texte final d'une dictée, et où il va : le deck (`text`) ou l'agent (`agent`). */
  onText: (text: string, mode: DictationMode) => void;
  onError: (message: string) => void;
  /** Un tour de l'agent est en cours (hors question ouverte) : ⌘⇧D l'arrête (Échap : voir `App`). */
  agentBusy: boolean;
  /** Arrête le tour de l'agent. */
  onAgentCancel: () => void;
}

/** L'appareil de dictée : micro, niveau, texte provisoire, réglages du modèle. */
export default function Dictation({ destination, prompt, onText, onError, agentBusy, onAgentCancel }: Props) {
  const [status, setStatus] = useState<SttStatus | null>(null);
  const [settings, setSettings] = useState(false);
  const [levels, setLevels] = useState<number[]>(() => Array(28).fill(0));
  const lastSeq = useRef<number | null>(null);
  const lastError = useRef<string | null>(null);
  const [mode, setMode] = useState<DictationMode>("text");
  // Lu par `receive` (stable) : la dictée qui se termine garde le mode de son début.
  const modeRef = useRef<DictationMode>("text");
  const handlers = useRef({ onText, onError, onAgentCancel });
  handlers.current = { onText, onError, onAgentCancel };

  const receive = useCallback((s: SttStatus) => {
    setStatus(s);
    if (lastSeq.current === null) lastSeq.current = s.seq;
    else if (s.seq !== lastSeq.current) {
      lastSeq.current = s.seq;
      if (s.text) handlers.current.onText(s.text, modeRef.current);
      else handlers.current.onError(t("ui-dictation-heard-nothing"));
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
    const timer = window.setInterval(() => send(stt.status()), status?.state === "downloading" ? 400 : 90);
    return () => window.clearInterval(timer);
  }, [busy, status?.state, send]);

  useEffect(() => {
    if (status?.state === "idle") setLevels((l) => l.map(() => 0));
  }, [status?.state]);

  const configure = (change: Partial<SttConfig>) => {
    if (status) send(stt.configure({ ...status.config, ...change }));
  };

  const toggle = useCallback(
    (next: DictationMode) => {
      if (!status) return;
      if (status.state === "idle") {
        setSettings(false);
        setMode(next);
        modeRef.current = next;
        send(stt.start(prompt || null));
      } else if (status.state === "recording") send(stt.stop());
    },
    [status, prompt, send],
  );

  // ⌘D texte, ⌘⇧D agent (Ctrl ailleurs), partout, même pendant la saisie d'un texte.
  // Écouteur posé une fois : l'état change toutes les 90 ms pendant une écoute,
  // il est lu par des refs.
  const keys = useRef({ status, agentBusy, toggle, send });
  keys.current = { status, agentBusy, toggle, send };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const { status: now, agentBusy: busy, toggle: switchMode, send: post } = keys.current;
      const shortcut = dictationShortcut(e, { agentBusy: busy, recording: now?.state === "recording" });
      if (shortcut) {
        e.preventDefault();
        e.stopPropagation();
        if (shortcut === "agent-cancel") handlers.current.onAgentCancel();
        else switchMode(shortcut);
      } else if (e.key === "Escape" && now && (now.state === "recording" || now.state === "downloading")) {
        // Échap arrête une écoute ; arrêter l'agent est l'affaire d'`App`, en dernier recours.
        e.preventDefault();
        e.stopPropagation();
        post(stt.cancel());
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);

  if (!status) {
    return (
      <div className="device object" aria-label={t("ui-dictation")}>
        <button className="mic" disabled aria-label={t("ui-dictation-start")}>{MicIcon}</button>
        <div className="spacer caption">{t("ui-dictation-preparing")}</div>
      </div>
    );
  }

  const model = status.models.find((m) => m.id === status.config.model);
  const langs = languages();
  const language = langs.find(([code]) => code === status.config.language)?.[1] ?? status.config.language;
  const { state } = status;
  // Le mode ne se montre que pendant une dictée : au repos, ⌘D et le micro vont au deck.
  const shownMode = state === "idle" ? "text" : mode;

  let body: React.ReactNode;
  if (state === "downloading" && status.download) {
    const d = status.download;
    const pct = Math.min(100, Math.floor((d.received / Math.max(1, d.total)) * 100));
    const name = status.models.find((m) => m.id === d.model)?.label.split(" (")[0];
    body = (
      <div className="dictation-body">
        <div className="dictation-line">
          <span className="dictation-state">{t("ui-dictation-downloading", { name: name ?? "" })}</span>
          <span className="caption">{t("ui-dictation-download-progress", { percent: pct, received: mo(d.received), total: mo(d.total) })}</span>
        </div>
        <div className="progress" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
          <span style={{ width: `${pct}%` }} />
        </div>
      </div>
    );
  } else if (state === "recording" || state === "transcribing") {
    const [done, current] = transcript(status.committed, status.partial);
    const detected = status.detected && !status.config.language ? langs.find(([c]) => c === status.detected)?.[1] ?? status.detected : null;
    let listening: React.ReactNode = <><span className={`rec-dot${status.speaking ? " voice" : ""}`} />{t("ui-dictation-listening", { time: clock(status.seconds) })}</>;
    if (state === "transcribing") listening = t("ui-dictation-transcribing");
    body = (
      <div className="dictation-body">
        <div className="dictation-line">
          <span className="dictation-state">{listening}</span>
          {state === "recording" && (
            <span className="dictation-meta">
              {status.stop_in !== null ? (
                <span className="caption stop-in">{t("ui-dictation-stop-in", { duration: secs(Math.ceil(status.stop_in * 2) / 2) })}</span>
              ) : detected ? (
                <span className="caption">{detected}</span>
              ) : null}
              <span className={`wave${status.speaking ? " voice" : ""}`} aria-hidden>
                {levels.map((l, i) => <span key={i} style={{ transform: `scaleY(${Math.max(0.08, Math.min(1, l * 1.6))})` }} />)}
              </span>
            </span>
          )}
        </div>
        <div className={`partial${done || current ? "" : " empty"}`} aria-live="polite">
          {done || current ? (
            <>
              {done}
              {done && current ? " " : ""}
              {current && <span className="tentative">{current}</span>}
            </>
          ) : state === "recording" ? t("ui-dictation-speak") : ""}
        </div>
      </div>
    );
  } else {
    body = (
      <div className="dictation-body">
        <div className="dictation-line">
          <span className="dictation-state">
            {agentBusy ? (
              t("ui-dictation-agent-working")
            ) : (
              <>{t("ui-dictation-start")} <span className="caption">→ {destination}</span></>
            )}
          </span>
          <span className="caption">
            {model?.label.split(" (")[0]} · {language}
            {status.config.auto_stop ? ` · ${t("ui-dictation-auto-stop", { duration: secs(status.config.silence) })}` : ""}
            {model && !model.installed ? ` · ${t("ui-dictation-to-download", { size: mo(model.size) })}` : ""}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className={`device object dictation ${state}${shownMode === "agent" ? " agent" : ""}`} aria-label={t("ui-dictation")}>
      <button
        className={`mic${state === "recording" ? " live" : ""}`}
        aria-label={t(state === "recording" ? "ui-dictation-finish" : "ui-dictation-start")}
        title={t(state === "recording" ? "ui-dictation-finish-title" : "ui-dictation-start-title")}
        disabled={state === "downloading" || state === "transcribing"}
        // Ne pas voler le focus : un texte en cours d'édition reste ouvert.
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => toggle("text")}
      >
        {state === "recording" ? StopIcon : MicIcon}
        {state === "recording" && <span className="mic-ring" style={{ transform: `scale(${1 + status.level * 0.5})` }} />}
      </button>
      {/* Au repos seulement : pendant une dictée, le micro principal l'arrête. */}
      <button
        type="button"
        className="mic agent-mic"
        aria-label={t(agentBusy ? "ui-dictation-agent-stop" : "ui-dictation-agent-start")}
        title={t(agentBusy ? "ui-dictation-agent-stop" : "ui-dictation-agent-start-title")}
        disabled={state !== "idle"}
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => (agentBusy ? handlers.current.onAgentCancel() : toggle("agent"))}
      >
        {agentBusy ? StopIcon : AgentIcon}
      </button>
      {body}
      {state === "recording" || state === "downloading" ? (
        <button className="btn ghost" onMouseDown={(e) => e.preventDefault()} onClick={() => send(stt.cancel())}>{t("ui-dictation-cancel")}</button>
      ) : (
        <button
          className="token ghost"
          aria-label={t("ui-dictation-settings")}
          aria-expanded={settings}
          disabled={state !== "idle"}
          onClick={() => setSettings((v) => !v)}
        >
          {GearIcon}
        </button>
      )}

      {settings && state === "idle" && (
        <div className="dictation-settings object" role="dialog" aria-label={t("ui-dictation-settings")}>
          <div className="settings-title">{t("ui-dictation-model-title")}</div>
          <p className="caption">{t("ui-dictation-model-intro")}</p>
          <div className="model-list" role="radiogroup">
            {status.models.map((m) => {
              const [name, detail] = m.label.replace(")", "").split(" (");
              return (
                <label key={m.id} className={`model${m.id === status.config.model ? " chosen" : ""}`}>
                  <input
                    type="radio"
                    name="stt-model"
                    checked={m.id === status.config.model}
                    onChange={() => configure({ model: m.id as SttModel })}
                  />
                  <span className="model-name">{name}</span>
                  <span className="caption">{modelNote(m.id)} · {detail?.replace(/^[^,]*, /, "")}</span>
                  <span className={`badge${m.installed ? " ok" : ""}`}>{t(m.installed ? "ui-dictation-installed" : "ui-dictation-not-installed")}</span>
                </label>
              );
            })}
          </div>
          <label className="settings-row">
            <span className="caption">{t("ui-dictation-language")}</span>
            <select
              className="select"
              value={status.config.language ?? ""}
              onChange={(e) => configure({ language: e.target.value || null })}
            >
              {langs.map(([code, name]) => <option key={code ?? "auto"} value={code ?? ""}>{name}</option>)}
            </select>
          </label>
          <div className="settings-row">
            <label className="check">
              <input type="checkbox" checked={status.config.auto_stop} onChange={(e) => configure({ auto_stop: e.target.checked })} />
              <span className="caption">{t("ui-dictation-auto-stop-label")}</span>
            </label>
            <select
              className="select"
              aria-label={t("ui-dictation-silence")}
              disabled={!status.config.auto_stop}
              value={String(status.config.silence)}
              onChange={(e) => configure({ silence: Number(e.target.value) })}
            >
              {(SILENCES.includes(status.config.silence) ? SILENCES : [...SILENCES, status.config.silence].sort((a, b) => a - b)).map((v) => (
                <option key={v} value={String(v)}>{secs(v)}</option>
              ))}
            </select>
          </div>
          <p className="caption">{t("ui-dictation-phrases-note")}</p>
          {model && !model.installed && (
            <button className="btn" onClick={() => send(stt.download(model.id))}>{t("ui-dictation-download-now", { size: mo(model.size) })}</button>
          )}
        </div>
      )}
    </div>
  );
}

/** Note d'un modèle de dictée ; aucune pour un modèle inconnu. */
function modelNote(id: string): string | undefined {
  switch (id) {
    case "turbo":
      return t("ui-dictation-note-turbo");
    case "large":
      return t("ui-dictation-note-large");
    case "base":
      return t("ui-dictation-note-base");
    default:
      return undefined;
  }
}
