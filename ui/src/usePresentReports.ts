// The presentation's host side: the page reports whether it presents, and takes the host's commands.
//
// The host reads these reports alone to know a tab presents (the agent's `present` steps, the
// spoken nav words). They share the window's page id and numbering with the context reports
// (`voiceContextStore`): a reload is a new page, whose reports start over on the host.

import { useEffect, useRef, useState } from "react";
import { agentApi, errorText } from "./api";
import type { PresentCommand, PresentMode, SlideCommand, SubscribeSlideCommands } from "./present";
import { voiceContext, type VoiceContextStore } from "./voiceContextStore";

/** Sends one presenting report of `tab` to the host; `mode` `null`: it stopped. */
export type ReportPresenting = (tab: string, page: string, mode: PresentMode | null, seq: number, window: string) => Promise<void>;

/** What the hook uses beyond the page: injectable for tests. */
export interface PresentReportsDeps {
  store?: VoiceContextStore;
  report?: ReportPresenting;
}

/** What the host's present commands do in this tab. */
export interface PresentWiring {
  /** A command of the host: `start` presents, the others steer the running presentation. */
  onCommand: (c: PresentCommand) => void;
  /** The steering commands, for the `Presenter`. */
  commands: SubscribeSlideCommands;
}

const sendPresenting: ReportPresenting = (tab, page, mode, seq, window) => agentApi(tab).presenting(page, mode, seq, window);

/**
 * Reports where window `window` presents tab `tab`, from an effect on `presenting`; that
 * effect's cleanup reports the end: the presentation left (Escape, a command, the system
 * leaving full screen), the tab switched or the page unmounted. A window destroyed while it
 * presents is cleared by the host itself.
 */
export function usePresentReports(
  tab: string,
  window: string | null,
  presenting: PresentMode | null,
  present: (mode: PresentMode) => void,
  deps: PresentReportsDeps = {},
): PresentWiring {
  const { store = voiceContext, report = sendPresenting } = deps;
  useEffect(() => {
    if (!presenting || !window) return;
    const send = (mode: PresentMode | null) =>
      void report(tab, store.page, mode, store.nextSeq(), window).catch((e) => console.warn("presentation report", errorText(e)));
    send(presenting);
    return () => send(null);
  }, [tab, window, presenting, store, report]);

  const [listeners] = useState(() => new Set<(c: SlideCommand) => void>());
  // A function state is an initializer: the subscriber goes in a wrapper.
  const [{ commands }] = useState(() => {
    const commands: SubscribeSlideCommands = (l) => {
      listeners.add(l);
      return () => void listeners.delete(l);
    };
    return { commands };
  });
  const presentNow = useRef(present);
  presentNow.current = present;
  const onCommand = (c: PresentCommand) => {
    if (c.action === "start") presentNow.current(c.mode);
    else for (const l of listeners) l(c);
  };
  return { onCommand, commands };
}
