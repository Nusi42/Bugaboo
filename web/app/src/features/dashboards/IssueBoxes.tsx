import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";

import { lineOf, SlimIssueList } from "../issues/index.ts";

/** No issue of the slim list is the open one: a box only lists them. */
const noneOpen = "";

/**
 * *Search results* (M10-F53): the first issues of the search as the slim list of the issue list draws them, with their
 * status, ID, priority and title; the answer has more members of each issue, which the box does not show.
 */
export function Results({ issues }: { readonly issues: readonly Schema<"IssueItem">[] }) {
  const lines = issues.flatMap((issue) => lineOf(issue) ?? []);
  return lines.length === 0 ? (
    <p {...partOf(part.muted)}>{t("dashboards.no-issues")}</p>
  ) : (
    <SlimIssueList lines={lines} open={noneOpen} />
  );
}
