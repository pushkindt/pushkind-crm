import { createRoot } from "react-dom/client";

import { NoAccessPage } from "../pages/NoAccessPage";

const ROOT_ELEMENT_ID = "react-root";

const rootElement = document.getElementById(ROOT_ELEMENT_ID);

if (!rootElement) {
  throw new Error(
    `Missing #${ROOT_ELEMENT_ID} mount node for the CRM no-access frontend.`,
  );
}

createRoot(rootElement).render(<NoAccessPage />);
