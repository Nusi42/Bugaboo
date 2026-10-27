import { t } from "@bugaboo/messages";

import type { QuickFound } from "./use-quick-results.ts";

/**
 * What a screen reader says when the results change: how many there are, or that nothing was found,
 * and that some results could not be loaded. Nothing while the server has yet to answer.
 */
export function announcement(found: QuickFound): string {
  if (found.waiting) {
    return "";
  }
  const count = found.groups.reduce((sum, group) => sum + group.results.length, 0);
  const said = count === 0 ? t("search.nothing") : t("search.count", { count });
  return found.failed ? `${said} ${t("search.partial")}` : said;
}
