import { applyInitialAppearance } from "@bugaboo/ui";
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "@bugaboo/ui/tokens.css";

import { paths } from "./shared/paths.ts";
import { onSecondStepRequired, onSessionLost, queryClient } from "./shared/query-client.ts";
import { forgetPerson, leaveSession } from "./shared/session.ts";
import { router } from "./routes/router.tsx";
import "./global.css";

applyInitialAppearance();

// An answer that says the sign-in is gone, or does not fit this page, takes the person to the
// sign-in page, and the cache forgets what the last person was allowed to see (M1-F51).
onSessionLost((reason) => {
  void leaveSession(async () => {
    // Already on the sign-in page: nothing to explain that is not shown there.
    if (router.state.location.pathname !== paths.signIn) {
      await router.navigate({ to: paths.signIn, search: { reason } });
    }
  });
});

// An answer that says the person has to set up a second step takes them to that screen, whatever
// page they were on (ADR-0025, decision 7).
onSecondStepRequired(() => {
  forgetPerson();
  if (router.state.location.pathname !== paths.setUpSecondStep) {
    void router.navigate({ to: paths.setUpSecondStep });
  }
});

const container = document.querySelector("#root");
if (container === null) {
  throw new Error("index.html must contain the element with id root");
}

createRoot(container).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
);
