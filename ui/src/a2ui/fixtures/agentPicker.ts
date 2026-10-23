// Le panneau des agents tel que l'émet l'hôte (évènement `picker`).

import type { Surface } from "../../api";

/** Copie du snapshot `picker_surface_idle_offers_every_preset_with_the_active_one_chosen` (crate oku-slides-agent). */
export const PICKER: Surface = {
  id: "agent-picker",
  place: "sheet",
  live: true,
  question: false,
  messages: [
    { version: "v0.9", createSurface: { surfaceId: "agent-picker", catalogId: "urn:oku-slides:a2ui:catalog:v4" } },
    {
      version: "v0.9",
      updateComponents: {
        surfaceId: "agent-picker",
        components: [
          { id: "root", component: "Column", children: ["title", "agents"] },
          { id: "title", component: "Text", text: "Agent", variant: "h4" },
          {
            id: "agents",
            component: "ChoiceChips",
            options: [
              { id: "claude", label: "Claude", hint: "npx -y claude-agent-acp" },
              { id: "gemini", label: "Gemini", hint: "npx -y gemini --acp" },
            ],
            value: { path: "/choice" },
            action: { event: { name: "select", context: { id: { path: "/choice" } } } },
          },
        ],
      },
    },
    { version: "v0.9", updateDataModel: { surfaceId: "agent-picker", value: { choice: "gemini" } } },
  ] as unknown as Surface["messages"],
};
