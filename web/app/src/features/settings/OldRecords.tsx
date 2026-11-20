import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, Card, ConfirmDialog, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../shared/errors.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { kindName, periodText, recordsText, type DataKind } from "./keep-rules.ts";
import { OldPersonLine } from "./OldPersonLine.tsx";
import { oldRecordsOptions, useDeleteOld } from "./old-records.ts";

type Old = Schema<"OldRecordsBody">;

/** What the old records are: how many, and since when. */
function Summary({ old }: { readonly old: Old }) {
  return (
    <p {...partOf(part.text)}>
      {old.total === 0
        ? t("old-records.none")
        : t("old-records.summary", {
            records: recordsText(old.kind, old.total),
            count: old.total,
            period: periodText(old.period),
            date: formatCalendarDay(old.before),
          })}
    </p>
  );
}

/** *Delete them all*, after a question that says what is deleted. */
function DeleteAll({
  old,
  busy,
  onDelete,
}: {
  readonly old: Old;
  readonly busy: boolean;
  readonly onDelete: () => void;
}) {
  const records = recordsText(old.kind, old.total);
  return (
    <div {...partOf(part.row)}>
      <ConfirmDialog
        triggerLabel={t("old-records.delete-all")}
        triggerLook={buttonLook.danger}
        title={t("old-records.delete-all-title", { records })}
        text={t("old-records.delete-all-text", { records, count: old.total })}
        confirmLabel={t("old-records.delete")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        confirmDisabled={busy}
        onConfirm={onDelete}
      />
    </div>
  );
}

/** What is past the keep period: the count, for each person, and *Delete them*. */
function OldList({ old }: { readonly old: Old }) {
  const { deleted, send, error, busy } = useDeleteOld(old.kind, old.before);
  return (
    <div {...partOf(part.stack)}>
      {error === undefined ? null : <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>}
      {deleted === undefined ? null : (
        <Alert tone={alertTone.success}>
          {t("old-records.deleted", { records: recordsText(old.kind, deleted), count: deleted })}
        </Alert>
      )}
      <Summary old={old} />
      {old.total === 0 ? null : (
        <>
          <ul {...partOf(part.list)}>
            {old.people.map((line) => (
              <OldPersonLine
                key={line.id ?? personName(line.person)}
                kind={old.kind}
                line={line}
                deleting={busy}
                onDelete={send}
              />
            ))}
          </ul>
          {old.truncated ? <p {...partOf(part.muted)}>{t("old-records.more-people")}</p> : null}
          <p {...partOf(part.muted)}>{t("old-records.nothing-by-itself")}</p>
          <DeleteAll
            old={old}
            busy={busy}
            onDelete={() => {
              send();
            }}
          />
        </>
      )}
    </div>
  );
}

/**
 * The records of a kind an administrator deletes (work days, time entries) that are older than the keep period,
 * counted when the page is read, for each person and in total, with *Delete them* for everybody or for one person after
 * a question in the page (ADR-0045, decision 6; ADR-0035, decision 7). The one part for every such kind: *Working
 * hours* and *Keep and delete rules* show it. The server lets only holders of *Change organization settings* ask.
 */
export function OldRecords({ kind }: { readonly kind: DataKind }) {
  const old = useQuery(oldRecordsOptions(kind));
  return (
    <Card heading={t("old-records.title", { kind: kindName(kind) })} intro={t("old-records.intro")}>
      {old.error === null ? null : <Alert tone={alertTone.danger}>{messageOf(old.error)}</Alert>}
      {old.data === undefined ? null : <OldList old={old.data} />}
    </Card>
  );
}
