import {
  CancelledError,
  MutationCache,
  notifyManager,
  QueryCache,
  QueryClient,
} from "@tanstack/react-query";

import { isSecondStepRequired, sessionLostReason, type SessionLostReason } from "./errors.ts";
import { isLeavingPage } from "./leaving-page.ts";

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

// The screen hears of changes of the cache in the turn they happen, not one timer later: a change
// writes its new value and starts *Saving…* in one turn, and both must reach the same frame, or
// what the change makes larger grows twice (UX §6.4).
notifyManager.setScheduler(queueMicrotask);

/**
 * The query cache of the app: the only place where data of the server is kept (ADR-0009). Asking
 * who is signed in is not a query of this cache but a plain request (`currentPerson`), because
 * on a guest page the answer "nobody" is the normal one.
 */
export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: lost,
    onSuccess: (answer) => {
      readHandler(answer);
    },
  }),
  mutationCache: new MutationCache({ onError: lost }),
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
      // No read starts while the page is being left (a reload, a link to another address, closing
      // it): the browser would cut it at once, and WebKit reports a read that starts after the
      // question to leave as an error of the page. A read with a condition of its own decides itself.
      enabled: () => !isLeavingPage(),
    },
  },
});

/**
 * Makes the reads that were held while the page was being left, once it stays (the person stayed, or
 * a download left the page where it was): those without an answer and those marked to be read again.
 * Reads that come back by themselves (every 30 seconds, on return to the tab) wait for their time.
 */
export function readHeldOnStaying(leaving: boolean): void {
  if (!leaving) {
    void queryClient.refetchQueries({
      type: "active",
      predicate: ({
        state,
      }: Readonly<{ state: Readonly<{ data: unknown; isInvalidated: boolean }> }>) =>
        state.data === undefined || state.isInvalidated,
    });
  }
}

let readHandler: (answer: unknown) => void = () => {
  // Until something wants to know what the server answered, nobody is told.
};

/**
 * Says who is told every answer the server gave to a question (not what the app puts in its cache
 * itself): the settings of the person use it to know the newest version the server has shown.
 */
export function onServerRead(handler: (answer: unknown) => void): void {
  readHandler = handler;
}

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
