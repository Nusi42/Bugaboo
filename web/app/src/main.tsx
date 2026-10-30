import { applyInitialAppearance } from "@bugaboo/ui";

import { installPersonalSources } from "./shared/personal-sources.ts";
import { paths } from "./shared/paths.ts";
import {
  ignoreCancelledQuestion,
  onSecondStepRequired,
  onSessionLost,
} from "./shared/query-client.ts";
import { forgetPerson, leaveSession } from "./shared/session.ts";
import { renderApp } from "./render-app.tsx";
import { router } from "./routes/router.tsx";

// The one style sheet, made from design/theme.kdl. It comes last, as the rules of the page frame
// always did: they win over a rule of a feature that has the same weight.
import "@bugaboo/ui/theme.css";

// The person's own choices come from the server once they are signed in; until then the page is
// drawn with the copy kept for the first frame, or the device's look.
installPersonalSources();
applyInitialAppearance();

// An answer that says the sign-in is gone, or does not fit this page, takes the person to the
// sign-in page, and the cache forgets what the last person was allowed to see (M1-F51).
window.addEventListener("unhandledrejection", ignoreCancelledQuestion);

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

renderApp();
