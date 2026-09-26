import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import Workspace from "./Workspace";
import { loadMessages } from "./i18n";
import "./styles.css";

// Les messages d'abord : chaque texte de l'interface en dépend. Sans hôte
// joignable, l'interface s'affiche quand même (identifiants des messages).
loadMessages()
  .catch((e) => console.error("messages de l'interface", e))
  .finally(() =>
    createRoot(document.getElementById("root")!).render(
      <StrictMode>
        <Workspace />
      </StrictMode>,
    ),
  );
