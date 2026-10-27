import { doNothing } from "../../shared/do-nothing.ts";
import type { QuickSearchAsker } from "./quick-search-sources.ts";

/** Somebody who asks quick search in a test: no permissions, and signing out does nothing. */
export const testAsker: QuickSearchAsker = { permissions: [], signOut: doNothing };
