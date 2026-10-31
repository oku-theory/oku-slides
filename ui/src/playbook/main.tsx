// Dev-only entry of the A2UI playbook, never part of the app build.
// Opened with `just web`, then /playbook.html.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import fixture from "../a2ui/fixtures/playbook.json";
import { applyDefaultScheme } from "../appearance";
import { loadMessages } from "../i18n";
import { currentScheme, loadScheme } from "../scheme";
import { Playbook } from "./Playbook";
import type { PlaybookEntry } from "./playbookEntries";
import "../styles.css";
import "../a2ui/a2ui.css";
import "./playbook.css";

Promise.all([
  loadMessages().catch((e) => console.error("interface messages", e)),
  loadScheme().catch((e) => console.error("interface scheme", e)),
]).finally(() => {
  applyDefaultScheme(currentScheme());
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <Playbook entries={fixture as unknown as PlaybookEntry[]} />
    </StrictMode>,
  );
});
