import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { DashboardBootstrap } from "../pages/DashboardBootstrap";
import "../styles/foundation.css";

const rootElement = document.getElementById("react-root");

if (!rootElement) {
  throw new Error("React root element not found.");
}

createRoot(rootElement).render(
  <StrictMode>
    <DashboardBootstrap />
  </StrictMode>,
);
