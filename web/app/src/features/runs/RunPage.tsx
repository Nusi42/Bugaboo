import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  IssueId,
  partOf,
  part,
  type LongPreviewGroup,
} from "@bugaboo/ui";
import { useState } from "react";

import { AppLongPreview } from "../../shared/app-long-preview.tsx";
import { changeLineText } from "../../shared/change-line.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshIssues } from "../../shared/query-keys.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import {
  runStatusText,
  runTitle,
  skipReasonText,
  useRun,
  useRunChanges,
  useUndoPreview,
  useUndoRun,
  type RunChange,
  type UndoPreview,
} from "./runs.ts";

/** The issues of one page of a run and what changed on each, in the words of the history. */
function ChangeList({ items }: { readonly items: readonly RunChange[] }) {
  return (
    <WithWording>
      {(wording) => (
        <ul {...partOf(part.list)}>
          {items.map((item) => (
            <li key={item.event} {...partOf(part.itemText)}>
              <strong>
                <IssueId id={item.issue.readable_id} />
              </strong>
              <ul {...partOf(part.list)}>
                {item.fields.map((field) => (
                  <li key={field.field}>{changeLineText(wording, field)}</li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      )}
    </WithWording>
  );
}

/** The changes of a run, a page at a time. */
function Changes({ run }: { readonly run: string }) {
  const [after, setAfter] = useState<string>();
  const read = useRunChanges(run, after);
  if (read.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert>;
  }
  const page = read.data;
  if (page === undefined) {
    return null;
  }
  const next = page.next ?? undefined;
  const empty = page.items.length === 0 && next === undefined && after === undefined;
  return (
    <section {...partOf(part.stack)} aria-label={t("runs.changes-heading")}>
      <h2>{t("runs.changes-heading")}</h2>
      {empty ? (
        <p {...partOf(part.muted)}>{t("runs.nothing-to-show")}</p>
      ) : (
        <ChangeList items={page.items} />
      )}
      {next === undefined ? null : (
        <div>
          <Button
            onPress={() => {
              setAfter(next);
            }}
          >
            {t("runs.show-more")}
          </Button>
        </div>
      )}
    </section>
  );
}

/** The fields *Undo the run* leaves as they are, as the one long preview of the app. */
function Preview({ preview }: { readonly preview: UndoPreview }) {
  const groups: readonly LongPreviewGroup[] = preview.skipped.map((skip, index) => ({
    key: `${skip.issue?.id ?? "x"}:${skip.field}:${String(index)}`,
    heading: (
      <strong>
        {skip.issue === undefined || skip.issue === null ? (
          t("runs.unknown-issue")
        ) : (
          <IssueId id={skip.issue.readable_id} />
        )}
      </strong>
    ),
    lines: [
      {
        key: "why",
        content: t("runs.skip-line", {
          issue: skip.issue?.readable_id ?? t("runs.unknown-issue"),
          field: skip.field,
          reason: skipReasonText(skip),
        }),
      },
    ],
    skipped: true,
  }));
  return (
    <AppLongPreview
      summary={t("runs.preview", { back: preview.goes_back, skipped: preview.skipped_count })}
      label={t("runs.skipped-list")}
      groups={groups}
    />
  );
}

/** *Undo the run*: look first, then undo with what was looked at. */
function Undo({ run, over }: { readonly run: string; readonly over: boolean }) {
  const [looking, setLooking] = useState(false);
  const preview = useUndoPreview(run, looking);
  const undo = useUndoRun();
  const done = undo.data;
  const shown = done !== undefined && done.changed_since_preview ? done : (done ?? preview.data);
  return (
    <section {...partOf(part.stack)}>
      {looking ? null : (
        <div>
          <Button
            isDisabled={!over}
            onPress={() => {
              setLooking(true);
            }}
          >
            {t("runs.look-first")}
          </Button>
        </div>
      )}
      {preview.isError ? <Alert tone={alertTone.danger}>{messageOf(preview.error)}</Alert> : null}
      {undo.isError ? <Alert tone={alertTone.danger}>{messageOf(undo.error)}</Alert> : null}
      {done?.changed_since_preview === true ? (
        <Alert tone={alertTone.warning}>{t("runs.changed-since-preview")}</Alert>
      ) : null}
      {shown === undefined ? null : <Preview preview={shown} />}
      {done !== undefined && !done.changed_since_preview ? (
        <Alert tone={alertTone.success}>
          {t("runs.undone", { back: done.goes_back, skipped: done.skipped_count })}
        </Alert>
      ) : null}
      {looking && shown !== undefined && (done === undefined || done.changed_since_preview) ? (
        <div>
          <Button
            isPending={undo.isPending}
            onPress={() => {
              undo.mutate(
                { params: { path: { run } }, body: { preview_version: shown.preview_version } },
                { onSuccess: refreshIssues },
              );
            }}
          >
            {t("runs.undo")}
          </Button>
        </div>
      ) : null}
    </section>
  );
}

/** A run: what it is of, how far it is, the changes in it and *Undo the run*. */
export function RunPage({ run }: { readonly run: string }) {
  const read = useRun(run);
  if (read.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert>;
  }
  const found = read.data;
  if (found === undefined) {
    return null;
  }
  return (
    <div {...partOf(part.stack)}>
      <h1>{runTitle(found)}</h1>
      <p {...partOf(part.muted)}>{t("runs.status", { status: runStatusText(found) })}</p>
      <Changes run={run} />
      <Undo run={run} over={found.status !== "running" && found.status !== "queued"} />
    </div>
  );
}
