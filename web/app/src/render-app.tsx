import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { isLeavingPage, shouldReport, watchLeavingPage } from "./shared/leaving-page.ts";
import { queryClient } from "./shared/query-client.ts";
import { router } from "./routes/router.tsx";

/**
 * What React does with an error that an error boundary caught, as it did before (with the stack of
 * components for the developer), and with one that nothing caught. Only a failed load of a part of the app
 * while the page is being left is left out (see `shouldReport`).
 */
function reportCaught(
  error: unknown,
  info: Readonly<{ componentStack?: string | undefined }>,
): void {
  if (shouldReport(error, isLeavingPage())) {
    console.error(error, info.componentStack);
  }
}

function reportUncaught(error: unknown): void {
  if (shouldReport(error, isLeavingPage())) {
    reportError(error);
  }
}

/** Draws the app into the element of the page that is made for it. */
export function renderApp(): void {
  const container = document.querySelector("#root");
  if (container === null) {
    throw new Error("index.html must contain the element with id root");
  }
  watchLeavingPage(window, {
    after: (run, ms) => window.setTimeout(run, ms),
    cancel: (timer) => {
      window.clearTimeout(timer);
    },
  });
  createRoot(container, {
    onCaughtError: reportCaught,
    onUncaughtError: reportUncaught,
  }).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </StrictMode>,
  );
}
