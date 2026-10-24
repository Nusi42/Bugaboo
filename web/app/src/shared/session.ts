import type { Schema } from "@bugaboo/api-client";

import { api } from "./api.ts";
import { forgetBrowserDrafts } from "./editing/browser-drafts.ts";
import { codeOf } from "./errors.ts";
import { mySettingsKey } from "./my-settings.ts";
import { forgetPersonalChoices } from "./personal-sources.ts";
import { queryClient } from "./query-client.ts";

/** The person who is signed in, with what they may do for the whole organization. */
export type Person = Schema<"MeBody">;

/**
 * Who is signed in. The cache only holds what the app was told (`forgetPerson`, `currentPerson`):
 * the query never asks the server by itself, so a page that shows the name cannot start a request
 * after the sign-in is gone.
 */
export function meOptions() {
  return api.query.queryOptions("get", "/api/v1/auth/me", undefined, {
    enabled: false,
    staleTime: Infinity,
  });
}

/** What the app needs to know before anybody signs in. */
export function statusOptions() {
  return api.query.queryOptions("get", "/api/v1/auth/status");
}

/**
 * The person who is signed in, or `null` when nobody is. This is a plain request and not a query
 * of the cache: on a guest page the answer "nobody" is normal and must not look like a lost
 * sign-in.
 */
export async function currentPerson(): Promise<Person | null> {
  const known = queryClient.getQueryData<Person>(meOptions().queryKey);
  if (known !== undefined) {
    return known;
  }
  try {
    const { data } = await api.client.GET("/api/v1/auth/me");
    if (data === undefined) {
      forgetPersonalChoices();
      return null;
    }
    queryClient.setQueryData(meOptions().queryKey, data);
    return data;
  } catch (error) {
    if (codeOf(error) === "auth.not-signed-in") {
      // Nobody is signed in: what the last person chose is not kept for the next one (M2-F13).
      forgetPersonalChoices();
      return null;
    }
    throw error;
  }
}

/** The ID of the person who is signed in as the app knows it now; `null` until the app knows. */
export function signedInPersonId(): string | null {
  return queryClient.getQueryData<Person>(meOptions().queryKey)?.id ?? null;
}

/**
 * Forgets who was known, after somebody signed in: the next page asks the server who it is and
 * what that person may do (the answer of signing in holds neither the permissions nor anything the
 * menus need).
 */
export function forgetPerson(): void {
  queryClient.removeQueries({ queryKey: meOptions().queryKey });
}

/** How long the questions on their way are waited for before the cache is emptied under them. */
const IN_FLIGHT_WAIT_MS = 5000;

function afterWait(): Promise<void> {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, IN_FLIGHT_WAIT_MS);
  });
}

/**
 * Waits until the questions to the server that are on their way are answered. Emptying the cache
 * under a question cancels it, and whoever waits for the answer (the guest layout, asking whether
 * the setup is open) is left with a cancellation instead of the page it was on its way to.
 */
async function answersOnTheirWay(): Promise<void> {
  const flying: Promise<unknown>[] = [];
  for (const query of queryClient.getQueryCache().findAll({ fetchStatus: "fetching" })) {
    if (query.promise !== undefined) {
      flying.push(query.promise);
    }
  }
  const waited = Promise.allSettled(flying);
  await Promise.race([waited, afterWait()]);
}

/**
 * Leaves the sign-in: the app forgets who it was, goes where `go` leads (the sign-in page), and
 * only then forgets everything else it knew about the server, so what one person was allowed to
 * see does not stay in memory for the next person at this browser. The cache is emptied after the
 * page has changed and the questions on their way are answered, because emptying it under pages that
 * still ask for their data makes those requests fail half way.
 */
export async function leaveSession(go: () => Promise<void>): Promise<void> {
  forgetPersonalChoices();
  queryClient.removeQueries({ queryKey: meOptions().queryKey });
  await go();
  // The page has changed, so nothing asks for the person's settings any more. They go now, not
  // with the cache: the look and the choice of theme of the page we are on read them, and for as
  // long as questions are on their way a choice made here would go to the person who has left.
  queryClient.removeQueries({ queryKey: mySettingsKey() });
  await answersOnTheirWay();
  queryClient.clear();
  // A change of the settings in the cache while the page was changing writes the copy for the
  // first frame again: forget it once more, now that nothing can write it.
  forgetPersonalChoices();
}

/**
 * Signs out on purpose (the person pressed *Sign out*): like {@link leaveSession}, and the texts
 * the browser kept as drafts go too, so the next person at this device finds none. A session that
 * is lost by itself keeps them: the same person finds them again after signing in.
 */
export async function signOutOnPurpose(go: () => Promise<void>): Promise<void> {
  forgetBrowserDrafts();
  await leaveSession(go);
}
