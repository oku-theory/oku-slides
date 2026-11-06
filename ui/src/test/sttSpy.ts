// A fake dictation engine for render tests: spies on `stt` (`voiceApi.ts`) and answers itself.

import { spyOn } from "bun:test";
import type { SttState, SttStatus } from "../voiceApi";
import { stt } from "../voiceApi";

/** A dictation state, idle and push-to-talk unless `change` says otherwise. */
export function sttStatus(change: Partial<SttStatus> = {}): SttStatus {
  return {
    state: "idle",
    config: { model: "turbo", language: null, auto_stop: false, silence: 2, version: 1, api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY", api_version: "", headers: [] }, hands_free: false, read_aloud: null, speech: { voice: null, rate: 1, in_presentation: false, barge_in: true } },
    models: [{ id: "turbo", label: "Large v3 Turbo (rapide)", size: 1, installed: true, available: true }],
    download: null,
    level: 0,
    speaking: false,
    committed: "",
    partial: "",
    detected: null,
    stop_in: null,
    text: null,
    seq: 0,
    seconds: 0,
    error: null,
    api_missing: null,
    suspended: false,
    ...change,
  };
}

/**
 * A fake engine: `start` records, `stop` returns `heard` (seq + 1) from a dictation and pauses
 * hands-free listening, `listen` listens, `cancel` goes idle. `calls` lists the requests.
 */
export function fakeStt(heard = "bonjour", change: Partial<SttStatus> = {}) {
  let current = sttStatus(change);
  const set = (state: SttState, change: Partial<SttStatus> = {}) => {
    current = { ...current, state, ...change };
    return Promise.resolve(current);
  };
  const calls: string[] = [];
  const starts: string[][] = [];
  // A held answer: the next `stt.status()` waits until the test gives it back.
  let held: Promise<SttStatus> | null = null;
  const statusSpy = spyOn(stt, "status").mockImplementation(() => {
    const p = held ?? Promise.resolve(current);
    held = null;
    return p;
  });
  const spies = [
    statusSpy,
    spyOn(stt, "start").mockImplementation((vocabulary: string[]) => (calls.push("start"), starts.push(vocabulary), set("recording"))),
    spyOn(stt, "listen").mockImplementation(() => (calls.push("listen"), set("listening"))),
    spyOn(stt, "stop").mockImplementation(() => {
      calls.push("stop");
      return current.state === "listening" ? set("idle") : set("idle", { seq: current.seq + 1, text: heard });
    }),
    spyOn(stt, "cancel").mockImplementation(() => (calls.push("cancel"), set("idle"))),
  ];
  return {
    calls,
    starts,
    statusCalls: () => statusSpy.mock.calls.length,
    current: (): SttStatus => current,
    /** Puts the engine in hands-free listening, the switch on. */
    listening: () => void set("listening", { config: { ...current.config, hands_free: true } }),
    /** Holds the next `stt.status()` answer; `resolve` gives it, merged over the current state. */
    defer(): { resolve(s: Partial<SttStatus>): void } {
      let resolve: (s: SttStatus) => void = () => {};
      held = new Promise((r) => (resolve = r));
      return { resolve: (change) => resolve({ ...current, ...change }) };
    },
    restore: () => spies.forEach((s) => s.mockRestore()),
  };
}
