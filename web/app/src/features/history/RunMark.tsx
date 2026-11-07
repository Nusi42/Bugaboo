import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { factSeparator, Link, partOf, part } from "@bugaboo/ui";

import { runAddress } from "../../shared/paths.ts";

/** What the mark says without the link: that the change undid a run, or was made in one. */
function markText(undid: boolean): string {
  return undid ? t("history.undid-run") : t("history.in-run");
}

/**
 * What ties a change to a run, as one line under its sentence: what it is, and the link to the run
 * it was made in or undid for a reader who may open it. The server leaves the run out for the
 * others, who see the mark alone.
 */
export function RunMark({ item }: { readonly item: Schema<"HistoryEntryItem"> }) {
  if (item.run_mark === undefined || item.run_mark === null) {
    return null;
  }
  const undid = item.run_mark === "undid";
  const run = item.run;
  if (run === undefined || run === null) {
    return <span {...partOf(part.historyRunMark)}>{markText(undid)}</span>;
  }
  const link = (
    <Link href={runAddress(run)}>
      {undid ? t("history.undid-run-link") : t("history.run-link")}
    </Link>
  );
  return (
    <span {...partOf(part.historyRunMark)}>
      {undid ? `${t("history.undid-run")} ${factSeparator} ` : null}
      {link}
    </span>
  );
}
