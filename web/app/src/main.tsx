import { t } from "@bugaboo/messages";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "@bugaboo/ui/tokens.css";

import { App } from "./App.tsx";
import "./global.css";

document.title = t("app.name");

const container = document.querySelector("#root");
if (container === null) {
  throw new Error("index.html must contain the element with id root");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
