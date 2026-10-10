import { useCallback, useEffect, useRef, useState } from "react";
import type { Surface, SttConfig, SttModel, SttStatus } from "./api";
import { stt } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import { AgentPicker } from "./agent/AgentPicker";
import type { DictationMode } from "./agent/keys";
import { dictationShortcut, shortcutAction, typingShortcut } from "./agent/keys";
import type { LinkShortcut } from "./agent/link";

const SILENCES = [1, 1.5, 2, 3, 5];

const secs = (s: number) => t("ui-seconds", { value: s });

const NAMED: [string, string][] = [
  ["fr", "Français"],
  ["en", "English"],
  ["de", "Deutsch"],
  ["es", "Español"],
  ["it", "Italiano"],
  ["pt", "Português"],
  ["nl", "Nederlands"],
];

/** Langues de la dictée, chacune dans sa langue ; « auto » dit celle retenue (`resolved`) s'il la connaît. */
const languages = (resolved: string | null = null): [string | null, string][] => {
  const name = resolved ? NAMED.find(([code]) => code === resolved)?.[1] : undefined;
  const auto = name ? t("ui-dictation-language-auto-resolved", { language: name }) : t("ui-dictation-language-auto");
  return [[null, auto], ...NAMED];
};

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
const GearIcon = (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></svg>
);

interface Props {
  /** Où ira le texte, affiché au repos (« dans le titre », « nouveau bloc »…). */
  destination: string;
  /** Mots utiles à la reconnaissance : titres, noms propres du deck. */
  vocabulary: string[];
  /** Texte final d'une dictée et son mode : l'app l'écrit au curseur ou au brouillon. */
  onText: (text: string, mode: DictationMode) => void;
  onError: (message: string) => void;
  /** Un tour de l'agent est en cours (hors question ouverte) : ⌘⇧D l'arrête (Échap : voir `App`). */
  agentBusy: boolean;
  /** Arrête le tour de l'agent. */
  onAgentCancel: () => void;
  /** Ouvre le brouillon pour écrire une consigne au clavier (⌘K). */
  onType: () => void;
  /**
   * Un raccourci relayé par la fenêtre principale (agent détaché). Un numéro plus grand que
   * le dernier vu agit une fois ; celui présent au montage n'agit pas.
   */
  remote?: { seq: number; key: LinkShortcut };
  /** Le panneau des agents reçu de l'hôte (`useAgentPicker`), en bas des réglages ; `null` : pas encore reçu. */
  agentPicker: Surface | null;
  /** Les réglages s'ouvrent : l'hôte renvoie le panneau des agents à jour. */
  onAgentPickerOpen: () => void;
  /** Un agent choisi ; les réglages restent ouverts. */
  onAgentSelect: (id: string) => void;
}

/** L'appareil de dictée : micro, clavier, niveau, texte provisoire, réglages du modèle et de l'agent. */
export default function Dictation({ destination, vocabulary, onText, onError, agentBusy, onAgentCancel, onType, remote, agentPicker, onAgentPickerOpen, onAgentSelect }: Props) {
  const [status, setStatus] = useState<SttStatus | null>(null);
  const [settings, setSettings] = useState(false);
  const [levels, setLevels] = useState<number[]>(() => Array(28).fill(0));
  const lastSeq = useRef<number | null>(null);
  const lastError = useRef<string | null>(null);
  const [mode, setMode] = useState<DictationMode>("auto");
  // Lu par `receive` (stable) : la dictée qui se termine garde le mode de son début.
  const modeRef = useRef<DictationMode>("auto");
  const handlers = useRef({ onText, onError, onAgentCancel, onType });
  handlers.current = { onText, onError, onAgentCancel, onType };

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
        send(stt.start(vocabulary));
      } else if (status.state === "recording") send(stt.stop());
    },
    [status, vocabulary, send],
  );

  // ⌘D, ⌘⇧D brouillon, ⌘K écrire (Ctrl ailleurs), partout, même pendant la saisie d'un texte.
  // Écouteur posé une fois : l'état change toutes les 90 ms pendant une écoute,
  // il est lu par des refs.
  const keys = useRef({ status, agentBusy, toggle, send });
  keys.current = { status, agentBusy, toggle, send };
  /** Ce que fait un raccourci de dictée, au clavier ou relayé par la fenêtre principale. */
  const applyShortcut = (key: LinkShortcut) => {
    const { status: now, agentBusy: busy, toggle: switchMode } = keys.current;
    if (key === "type") return handlers.current.onType();
    const action = shortcutAction(key, { agentBusy: busy, recording: now?.state === "recording" });
    if (action === "agent-cancel") handlers.current.onAgentCancel();
    else switchMode(action);
  };

  // Un raccourci relayé (agent détaché) : chaque numéro neuf agit une fois ; celui présent au montage, jamais.
  const seen = useRef(remote?.seq ?? 0);
  useEffect(() => {
    if (!remote || remote.seq <= seen.current) return;
    seen.current = remote.seq;
    // `applyShortcut` ne lit que des refs (`keys`, `handlers`) : sa version courante suffit.
    applyShortcut(remote.key);
  }, [remote?.seq]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const { status: now, agentBusy: busy, send: post } = keys.current;
      const shortcut = dictationShortcut(e, { agentBusy: busy, recording: now?.state === "recording" });
      if (typingShortcut(e)) {
        e.preventDefault();
        e.stopPropagation();
        applyShortcut("type");
      } else if (shortcut) {
        e.preventDefault();
        e.stopPropagation();
        applyShortcut(e.shiftKey ? "draft" : "auto");
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

  // Écrire au lieu de dicter : ne dépend pas du moteur de dictée, toujours là.
  const typeButton = (
    <button
      type="button"
      className="token ghost dictation-type"
      aria-label={t("ui-dictation-type")}
      title={t("ui-dictation-type-title")}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => handlers.current.onType()}
    >
      {Icon.keyboard}
    </button>
  );

  if (!status) {
    return (
      <div className="device object" aria-label={t("ui-dictation")}>
        <button className="mic" disabled aria-label={t("ui-dictation-start")}>{MicIcon}</button>
        {typeButton}
        <div className="spacer caption">{t("ui-dictation-preparing")}</div>
      </div>
    );
  }

  const model = status.models.find((m) => m.id === status.config.model);
  const langs = languages(status.config.model === "apple" ? status.detected : null);
  const language = langs.find(([code]) => code === status.config.language)?.[1] ?? status.config.language;
  const { state } = status;
  // Le mode ne se montre que pendant une dictée : au repos, le micro est en mode auto.
  const shownMode = state === "idle" ? "auto" : mode;

  let body: React.ReactNode;
  // Détail du repos (destination, modèle, arrêt auto), porté par l'infobulle du micro.
  let idleDetails = "";
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
    // Au repos, rien que le micro : où va le texte et les réglages sont dans son
    // infobulle ; seuls l'agent au travail et un modèle à télécharger (ou
    // indisponible pour cette langue) s'écrivent.
    idleDetails = [
      `→ ${destination}`,
      [model?.label.split(" (")[0], language].filter(Boolean).join(" · "),
      status.config.auto_stop ? t("ui-dictation-auto-stop", { duration: secs(status.config.silence) }) : "",
    ].filter(Boolean).join("\n");
    const toDownload = model && model.available && !model.installed;
    const unavailable = model && !model.available;
    body = (agentBusy || toDownload || unavailable) && (
      <div className="dictation-body">
        {agentBusy && (
          // L'état en clair, le raccourci pour l'arrêter en dessous, en petit.
          <div className="dictation-agent">
            <span className="dictation-state">{t("ui-dictation-agent-working")}</span>
            <span className="caption dictation-hint">{t("ui-dictation-agent-working-hint")}</span>
          </div>
        )}
        {toDownload && (
          <div className="dictation-line">
            <span className="caption">{t(model.size > 0 ? "ui-dictation-to-download" : "ui-dictation-to-download-language", { size: mo(model.size) })}</span>
          </div>
        )}
        {unavailable && (
          <div className="dictation-line">
            <span className="caption">{t("ui-dictation-model-unavailable")}</span>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className={`device object dictation ${state}${shownMode === "draft" ? " draft" : ""}`} aria-label={t("ui-dictation")}>
      <button
        className={`mic${state === "recording" ? " live" : ""}`}
        aria-label={t(state === "recording" ? "ui-dictation-finish" : "ui-dictation-start")}
        title={state === "recording" ? t("ui-dictation-finish-title") : [t("ui-dictation-start-title"), idleDetails].filter(Boolean).join("\n")}
        disabled={state === "downloading" || state === "transcribing"}
        // Ne pas voler le focus : un texte en cours d'édition reste ouvert.
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => toggle("auto")}
      >
        {state === "recording" ? StopIcon : MicIcon}
        {state === "recording" && <span className="mic-ring" style={{ transform: `scale(${1 + status.level * 0.5})` }} />}
      </button>
      {/* Un seul micro : ce bouton n'apparaît que pour arrêter un tour de l'agent. */}
      {agentBusy && (
        <button
          type="button"
          className="mic agent-mic"
          aria-label={t("ui-dictation-agent-stop")}
          title={t("ui-dictation-agent-stop")}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => handlers.current.onAgentCancel()}
        >
          {StopIcon}
        </button>
      )}
      {typeButton}
      {body}
      {state === "recording" || state === "downloading" ? (
        <button className="token ghost dictation-cancel" aria-label={t("ui-dictation-cancel")} title={t("ui-dictation-cancel")} onMouseDown={(e) => e.preventDefault()} onClick={() => send(stt.cancel())}>{Icon.close}</button>
      ) : (
        <button
          className="token ghost"
          aria-label={t("ui-dictation-settings")}
          aria-expanded={settings}
          disabled={state !== "idle"}
          onClick={() => {
            if (!settings) onAgentPickerOpen();
            setSettings(!settings);
          }}
        >
          {GearIcon}
        </button>
      )}

      {settings && state === "idle" && (
        <div className="dictation-settings object" role="dialog" aria-label={t("ui-dictation-settings")}>
          <div className="settings-title">{t("ui-dictation-model-title")}</div>
          <p className="caption">{t(status.models.some((m) => m.id === "apple") ? "ui-dictation-model-intro-apple" : "ui-dictation-model-intro")}</p>
          <div className="model-list" role="radiogroup">
            {status.models.map((m) => {
              const [name, detail] = m.label.replace(")", "").split(" (");
              return (
                <label key={m.id} className={`model${m.id === status.config.model ? " chosen" : ""}`}>
                  <input
                    type="radio"
                    name="stt-model"
                    checked={m.id === status.config.model}
                    disabled={!m.available}
                    onChange={() => configure({ model: m.id as SttModel })}
                  />
                  <span className="model-name">{name}</span>
                  <span className="caption">{modelNote(m.id)}{m.size > 0 && ` · ${detail?.replace(/^[^,]*, /, "") ?? ""}`}</span>
                  <span className={`badge${m.available && m.installed ? " ok" : ""}`}>
                    {t(!m.available ? "ui-dictation-model-unavailable" : m.installed ? "ui-dictation-installed" : "ui-dictation-not-installed")}
                  </span>
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
          {model && model.available && !model.installed && (
            <button className="btn" onClick={() => send(stt.download(model.id))}>{model.size === 0 ? t("ui-dictation-download-language") : t("ui-dictation-download-now", { size: mo(model.size) })}</button>
          )}
          {/* L'agent de l'app, sous les réglages de la dictée (la surface porte son titre). */}
          {agentPicker && (
            <div className="dictation-agents">
              <AgentPicker surface={agentPicker} onSelect={onAgentSelect} onError={onError} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/** Note d'un modèle de dictée ; aucune pour un modèle inconnu. */
function modelNote(id: string): string | undefined {
  switch (id) {
    case "apple":
      return t("ui-dictation-note-apple");
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
