import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { factSeparator, Link } from "@bugaboo/ui";

import { runAddress } from "../../shared/paths.ts";

/** What stands before the link to the run: that the change undid one. */
function lead(undid: boolean): string {
  return undid
    ? ` ${factSeparator} ${t("history.undid-run")} ${factSeparator} `
    : ` ${factSeparator} `;
}

/** What ties a change to a run: that it undid one, and the link to the run it was made in or undid. */
export function RunMark({ item }: { readonly item: Schema<"HistoryEntryItem"> }) {
  const run = item.undid_run ?? item.run;
  if (run === undefined || run === null) {
    return null;
  }
  const undid = item.undid_run !== undefined && item.undid_run !== null;
  return (
    <>
      {lead(undid)}
      <Link href={runAddress(run)}>{t("history.run-link")}</Link>
    </>
  );
}
