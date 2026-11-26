import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { IssueId, part, partOf } from "@bugaboo/ui";

import { ChangeEntry } from "../history/index.ts";

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
