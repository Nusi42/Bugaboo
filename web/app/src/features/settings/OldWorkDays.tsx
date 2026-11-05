import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, Card, ConfirmDialog, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../shared/errors.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { OldPersonLine } from "./OldPersonLine.tsx";
import { oldDaysOptions, useDeleteOld } from "./old-work-days.ts";

type Old = Schema<"OldWorkDaysBody">;

/** What the old work days are: how many, and since when. */
function Summary({ old }: { readonly old: Old }) {
  return (
    <p>
      {old.total === 0
        ? t("old-work-days.none")
        : t("old-work-days.summary", {
            count: old.total,
            years: old.keep_years,
            date: formatCalendarDay(old.before),
          })}
    </p>
  );
}

/** *Delete them all*, after a question that says what is deleted. */
function DeleteAll({
  total,
  busy,
  onDelete,
}: {
  readonly total: number;
  readonly busy: boolean;
  readonly onDelete: () => void;
}) {
  return (
    <div {...partOf(part.row)}>
      <ConfirmDialog
        triggerLabel={t("old-work-days.delete-all")}
        title={t("old-work-days.delete-all-title", { count: total })}
        text={t("old-work-days.delete-all-text", { count: total })}
        confirmLabel={t("old-work-days.delete")}
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
  const { deleted, send, error, busy } = useDeleteOld();
  return (
    <div {...partOf(part.stack)}>
      {error === undefined ? null : <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>}
      {deleted === undefined ? null : (
        <Alert tone={alertTone.success}>{t("old-work-days.deleted", { count: deleted })}</Alert>
      )}
      <Summary old={old} />
      {old.total === 0 ? null : (
        <>
          <ul {...partOf(part.list)}>
            {old.people.map((line) => (
              <OldPersonLine
                key={line.id ?? personName(line.person)}
                line={line}
                deleting={busy}
                onDelete={send}
              />
            ))}
          </ul>
          {old.truncated ? <p {...partOf(part.muted)}>{t("old-work-days.more-people")}</p> : null}
          <p {...partOf(part.muted)}>{t("old-work-days.nothing-by-itself")}</p>
          <DeleteAll
            total={old.total}
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
 * The work days older than the keep period, counted when the page is read, for each person and in
 * total, with *Delete them* for everybody or for one person after a question in the page (ADR-0035,
 * decision 7). The server lets only holders of *Change organization settings* ask.
 */
export function OldWorkDays() {
  const old = useQuery(oldDaysOptions());
  return (
    <Card heading={t("old-work-days.title")} intro={t("old-work-days.intro")}>
      {old.error === null ? null : <Alert tone={alertTone.danger}>{messageOf(old.error)}</Alert>}
      {old.data === undefined ? null : <OldList old={old.data} />}
    </Card>
  );
}
