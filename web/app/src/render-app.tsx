import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { isLeavingPage, watchLeavingPage } from "./shared/leaving-page.ts";
import { queryClient } from "./shared/query-client.ts";
import { router } from "./routes/router.tsx";

/**
 * A part of the app that is still being fetched when the page is reloaded or left is cancelled by the
 * browser: the error that follows is no defect and is not reported. Any other error is reported as before.
 */
function reportUnlessLeaving(error: unknown): void {
  if (!isLeavingPage()) {
    reportError(error);
  }
}

/** Draws the app into the element of the page that is made for it. */
export function renderApp(): void {
  const container = document.querySelector("#root");
  if (container === null) {
    throw new Error("index.html must contain the element with id root");
  }
  watchLeavingPage(window, (restore, after) => {
    window.setTimeout(restore, after);
  });
  createRoot(container, {
    onCaughtError: reportUnlessLeaving,
    onUncaughtError: reportUnlessLeaving,
  }).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </StrictMode>,
  );
}
