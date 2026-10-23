import { CancelledError, MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";

import { isSecondStepRequired, sessionLostReason, type SessionLostReason } from "./errors.ts";

let sessionLostHandler: (reason: SessionLostReason) => void = () => {
  // Until the router is there, nobody can be taken anywhere.
};

/**
 * Says what happens when any answer shows that the person is not signed in any more, or that
 * the sign-in does not fit this page (for example after the server lost its CSRF key, M1-F51):
 * the person is taken to the sign-in page, not left on a page that cannot work, and the page is
 * not simply reloaded.
 */
export function onSessionLost(handler: (reason: SessionLostReason) => void): void {
  sessionLostHandler = handler;
}

let secondStepHandler: () => void = () => {
  // Until the router is there, nobody can be taken anywhere.
};

/**
 * Says what happens when any answer shows that the person has to set up a second step before
 * anything else (a role of theirs, or the organization, asks for one from now on): the person is
 * taken to that screen, also from a page that was open before.
 */
export function onSecondStepRequired(handler: () => void): void {
  secondStepHandler = handler;
}

function lost(error: unknown): void {
  const reason = sessionLostReason(error);
  if (reason !== undefined) {
    sessionLostHandler(reason);
  } else if (isSecondStepRequired(error)) {
    secondStepHandler();
  }
}

/**
 * The query cache of the app: the only place where data of the server is kept (ADR-0009). Asking
 * who is signed in is not a query of this cache but a plain request (`currentPerson`), because
 * on a guest page the answer "nobody" is the normal one.
 */
export const queryClient = new QueryClient({
  queryCache: new QueryCache({ onError: lost }),
  mutationCache: new MutationCache({ onError: lost }),
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});

/**
 * A question that was cancelled (the cache was emptied under it when somebody signed this place
 * out) is expected, not a defect: nobody is waiting for its answer any more. The browser reports a
 * rejection that nobody handles as an error of the app; this says that this one is none.
 */
export function ignoreCancelledQuestion(
  event: Readonly<{ reason: unknown; preventDefault: () => void }>,
): void {
  if (event.reason instanceof CancelledError) {
    event.preventDefault();
  }
}
