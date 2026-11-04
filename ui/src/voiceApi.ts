// Dictation on the host: its state, its settings and the requests the page sends.
// Hands-free listening (`listening`) hands each utterance to the host, never to the page.

import { host } from "./hostCall";

export type SttModel = "apple" | "turbo" | "large" | "base" | "api";
/** `listening`: hands-free, each utterance goes to the host, never to the page. */
export type SttState = "idle" | "downloading" | "recording" | "transcribing" | "listening";

/** OpenAI or Azure OpenAI dictation service; never the key here, only its variable's name. */
export interface SttApiConfig {
  url: string;
  model: string;
  /** Name of the variable holding the key, required; never the key itself. */
  key_env: string;
  /** Azure OpenAI `api-version`; empty: the OpenAI API. */
  api_version: string;
  /** Custom headers; an `env` value names a variable, never its value. */
  headers: { name: string; value: { literal: string } | { env: string } }[];
}

/** The read-aloud settings besides the switch, saved with the dictation's. */
export interface SttSpeechConfig {
  /** The chosen OS voice id; null: the first voice of the interface language. */
  voice: string | null;
  /** Speaking rate, a factor of the OS default, from 0.5 to 2. */
  rate: number;
  /** Read the answer to a wake-phrase request while the tab presents. */
  in_presentation: boolean;
}

export interface SttConfig {
  model: SttModel;
  language: string | null;
  /** Stops listening by itself after `silence` seconds of silence. */
  auto_stop: boolean;
  silence: number;
  /** Settings version (kept by the host). */
  version: number;
  api: SttApiConfig;
  /** Hands-free listening, chosen by the user only; every launch starts paused. */
  hands_free: boolean;
  /** Read answers aloud; null until set, turned on when hands-free is first turned on. */
  read_aloud: boolean | null;
  /** The read-aloud voice, rate and presentation rule; written by the host only. */
  speech: SttSpeechConfig;
}

export interface SttStatus {
  state: SttState;
  /** First API variable not set, by name only. */
  api_missing: string | null;
  config: SttConfig;
  /** `available` false: the model does not handle the chosen language. */
  models: { id: SttModel; label: string; size: number; installed: boolean; available: boolean }[];
  download: { model: SttModel; received: number; total: number } | null;
  level: number;
  /** Voice detected right now. */
  speaking: boolean;
  /** Sentences already transcribed (final) and the sentence in progress (preview). */
  committed: string;
  partial: string;
  detected: string | null;
  /** Seconds before the automatic stop, during a silence. */
  stop_in: number | null;
  text: string | null;
  seq: number;
  seconds: number;
  error: string | null;
  /** Hands-free listening is suspended (Oku is speaking). */
  suspended: boolean;
}

type SttRequest =
  | { cmd: "status" }
  | { cmd: "start"; vocabulary: string[] }
  | { cmd: "listen"; vocabulary: string[] }
  | { cmd: "stop" }
  | { cmd: "cancel" };

const sttCall = (req: SttRequest) => host<SttStatus>("stt", { req }, "/api/stt", req);

/** Dictation requests: each returns the state that follows. */
export const stt = {
  status: () => sttCall({ cmd: "status" }),
  start: (vocabulary: string[]) => sttCall({ cmd: "start", vocabulary }),
  /** Starts hands-free listening; a no-op while listening. */
  listen: (vocabulary: string[]) => sttCall({ cmd: "listen", vocabulary }),
  /** Ends a dictation, or pauses hands-free listening. */
  stop: () => sttCall({ cmd: "stop" }),
  cancel: () => sttCall({ cmd: "cancel" }),
};

/** Hands-free listening runs: each utterance goes to the host. */
export const listening = (s: SttStatus): boolean => s.state === "listening";
