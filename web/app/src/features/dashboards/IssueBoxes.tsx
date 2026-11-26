import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { IssueId, part, partOf } from "@bugaboo/ui";

import { ChangeEntry } from "../history/index.ts";
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

/** What tells the changes of the box apart: the issue and the event of its first entry. */
function changeKey(change: Schema<"RecentChangeBody">): string {
  return [change.issue, change.entries[0]?.event ?? ""].join(" ");
}

/** *Recent changes*: the newest changes of the issues of the search, each issue with its entries as its history words them. */
export function Changes({ changes }: { readonly changes: readonly Schema<"RecentChangeBody">[] }) {
  if (changes.length === 0) {
    return <p {...partOf(part.muted)}>{t("dashboards.no-changes")}</p>;
  }
  return (
    <ul {...partOf(part.list)}>
      {changes.map((change) => (
        <li key={changeKey(change)}>
          <IssueId id={change.issue} />
          <ChangeEntry items={change.entries} />
        </li>
      ))}
    </ul>
  );
}
