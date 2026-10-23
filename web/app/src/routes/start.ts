import { CancelledError } from "@tanstack/react-query";

import { queryClient } from "../shared/query-client.ts";
import { paths } from "../shared/paths.ts";
import { statusOptions } from "../shared/session.ts";

/** How long what the server said about itself before anybody signs in is trusted. */
const STATUS_STALE_MS = 5000;

function askStatus() {
  return queryClient.query({ ...statusOptions(), staleTime: STATUS_STALE_MS });
}

/**
 * What the app needs to know before anybody signs in: whether the first-start setup is open,
 * whether people may sign up, whether passkeys work.
 */
export async function loadStatus() {
  try {
    return await askStatus();
  } catch (error) {
    // The cache was emptied while the question was on its way (somebody signed this place out): a
    // cancelled question is asked again, not told to the page as a fault.
    if (error instanceof CancelledError) {
      return askStatus();
    }
    throw error;
  }
}

/** Where somebody who is not signed in is taken: to the setup while it is open, else to sign in. */
export async function startPath(): Promise<typeof paths.setup | typeof paths.signIn> {
  return (await loadStatus()).setup_needed ? paths.setup : paths.signIn;
}
