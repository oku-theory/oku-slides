import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import AgentWindow from "./AgentWindow";
import Workspace from "./Workspace";
import { isAgentWindow } from "./agent/target";
import { followLocale, loadMessages, onLocaleChange } from "./i18n";
import { applyDefaultScheme } from "./appearance";
import { applyLens } from "./lens";
import { currentScheme, followScheme, loadScheme } from "./scheme";
import "./styles.css";
import "./import.css";
import "./pack.css";
import "./a2ui/a2ui.css";
import "./glass.css";

// Les messages d'abord : chaque texte de l'interface en dépend. Sans hôte
// joignable, l'interface s'affiche quand même (identifiants des messages).
// Le schéma choisi en même temps : pas d'éclair clair avant le sombre. Puis
// chaque fenêtre suit le schéma et la langue choisis dans une autre.
followScheme();
followLocale();
Promise.all([
  loadMessages().catch((e) => console.error("messages de l'interface", e)),
  loadScheme().catch((e) => console.error("schéma de l'interface", e)),
])
  .finally(() => {
    applyDefaultScheme(currentScheme());
    applyLens(document.documentElement, navigator.userAgent);
    const agentWindow = isAgentWindow();
    const root = createRoot(document.getElementById("root")!);
    // Une autre langue : l'interface est remontée, chaque texte relu.
    let generation = 0;
    const render = () =>
      root.render(
        <StrictMode>
          {agentWindow ? <AgentWindow key={generation++} /> : <Workspace key={generation++} />}
        </StrictMode>,
      );
    onLocaleChange(render);
    render();
  });
