import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import Workspace from "./Workspace";
import { loadMessages, onLocaleChange } from "./i18n";
import "./styles.css";

// Les messages d'abord : chaque texte de l'interface en dépend. Sans hôte
// joignable, l'interface s'affiche quand même (identifiants des messages).
loadMessages()
  .catch((e) => console.error("messages de l'interface", e))
  .finally(() => {
    const root = createRoot(document.getElementById("root")!);
    // Une autre langue : l'interface est remontée, chaque texte relu.
    let generation = 0;
    const render = () =>
      root.render(
        <StrictMode>
          <Workspace key={generation++} />
        </StrictMode>,
      );
    onLocaleChange(render);
    render();
  });
