import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { ManagersBootstrap } from "../pages/ManagersBootstrap";
import "../styles/foundation.css";

const rootElement = document.getElementById("react-root");

if (!rootElement) {
  throw new Error("React root element not found.");
}

createRoot(rootElement).render(
  <StrictMode>
    <ManagersBootstrap />
  </StrictMode>,
);
