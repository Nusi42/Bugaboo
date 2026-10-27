import { t } from "@bugaboo/messages";

import type { QuickFound } from "./use-quick-results.ts";

/**
 * The sentences a screen reader says when the results change: how many there are, or that nothing
 * was found, and that some results could not be loaded. Each is a catalog sentence of its own, said
 * one after the other. None while the server has yet to answer.
 */
export function announcement(found: QuickFound): readonly string[] {
  if (found.waiting) {
    return [];
  }
  const count = found.groups.reduce((sum, group) => sum + group.results.length, 0);
  const said = count === 0 ? t("search.nothing") : t("search.count", { count });
  return found.failed ? [said, t("search.partial")] : [said];
}
