import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, IssueId, Link, partOf, part } from "@bugaboo/ui";

import { changeArrowText } from "../../shared/change-line.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
import { issueAddress } from "../../shared/paths.ts";
import { itemsOf } from "../../shared/paged.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { outcomeText, useRunChanges, type RunChange, type RunField } from "./runs.ts";

/** The issue of a change: its ID as a link to it, and what it is about. */
function IssueCell({ issue }: { readonly issue: RunChange["issue"] }) {
  return (
    <>
      <Link href={issueAddress(issue.readable_id)}>
        <IssueId id={issue.readable_id} />
      </Link>
      {issue.title === undefined || issue.title === null ? null : <div>{issue.title}</div>}
    </>
  );
}

/** What tells the rows of the table apart: the event and the field. */
function rowKey(item: RunChange, field: RunField): string {
  return [item.event, field.field].join("/");
}

/** One row for each field that changed: the issue, what changed (and what became of it), and when. */
function ChangeRows({ items }: { readonly items: readonly Readonly<RunChange>[] }) {
  return (
    <WithWording>
      {(wording) => (
        <tbody>
          {items.flatMap((item) =>
            item.changes.map((field) => {
              const outcome = outcomeText(field.outcome);
              return (
                <tr key={rowKey(item, field)}>
                  <td {...partOf(part.runTableCell)}>
                    <IssueCell issue={item.issue} />
                  </td>
                  <td {...partOf(part.runTableCell)}>
                    <div>{changeArrowText(wording, field)}</div>
                    {outcome === undefined ? null : <div {...partOf(part.muted)}>{outcome}</div>}
                  </td>
                  <td {...partOf(part.runTableCell, { variants: ["when"] })}>
                    {formatDateTime(item.at)}
                  </td>
                </tr>
              );
            }),
          )}
        </tbody>
      )}
    </WithWording>
  );
}

/** The changes of a run as a table, a page at a time. */
export function RunChanges({ run }: { readonly run: string }) {
  const read = useRunChanges(run);
  if (read.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert>;
  }
  if (read.data === undefined) {
    return null;
  }
  const items = itemsOf(read.data.pages);
  return (
    <Card heading={t("runs.changes-heading")}>
      {items.length === 0 ? (
        <p {...partOf(part.muted)}>{t("runs.nothing-to-show")}</p>
      ) : (
        <table {...partOf(part.runTable)}>
          <thead>
            <tr>
              <th {...partOf(part.runTableHead)}>{t("runs.column-issue")}</th>
              <th {...partOf(part.runTableHead)}>{t("runs.column-what")}</th>
              <th {...partOf(part.runTableHead)}>{t("runs.column-when")}</th>
            </tr>
          </thead>
          <ChangeRows items={items} />
        </table>
      )}
      {read.hasNextPage ? (
        <div>
          <Button
            isPending={read.isFetchingNextPage}
            onPress={() => {
              void read.fetchNextPage();
            }}
          >
            {t("runs.show-more")}
          </Button>
        </div>
      ) : null}
    </Card>
  );
}
