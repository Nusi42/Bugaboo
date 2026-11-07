import { t } from "@bugaboo/messages";
import { Button, buttonLook, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useId, useState } from "react";

import type { TimeEntry } from "./edits.ts";
import type { EntryIssue } from "./entry-issue.ts";
import { EntryChangeForm, type EntryChangeFormProps } from "./EntryChangeForm.tsx";
import { EntryList, type EntryListProps } from "./EntryList.tsx";
import { EntryWindow } from "./EntryWindow.tsx";
import { useChangeEntry } from "./use-change-entry.ts";
import { useDetailsTitle } from "./details-title.ts";

export interface EntryDetailsWindowProps {
  readonly issue: EntryIssue;
  /** The entries of the issue in the week, in the order of their days. */
  readonly entries: readonly TimeEntry[];
  readonly defaults: EntryChangeFormProps["defaults"];
  readonly wording: EntryListProps["wording"];
  readonly onClose: () => void;
}

/** The foot of the window: *Close* of the list, or *Back to the list* and *Save change* of one entry. */
function DetailsFooter({
  formId,
  picked,
  isPending,
  onClose,
  onBack,
}: {
  readonly formId: string;
  readonly picked: boolean;
  readonly isPending: boolean;
  readonly onClose: () => void;
  readonly onBack: () => void;
}) {
  return (
    <div {...partOf(part.windowFooterActions)}>
      <Button look={buttonLook.quiet} onPress={picked ? onBack : onClose}>
        {picked ? t("time.details.back") : t("common.close")}
      </Button>
      {picked ? (
        <SubmitButton form={formId} isPending={isPending}>
          {t("time.details.save")}
        </SubmitButton>
      ) : null}
    </div>
  );
}

/**
 * The details of the time on one issue in the week (TIME-04): every entry of it, so a cell that
 * holds several can be changed one by one, and for each the same form as *Add time* and *Stop
 * timer* (`EntryForm`) in the same window (`EntryWindow`), where the length, the note, *Billable*
 * and the fields of an entry are changed.
 */
export function EntryDetailsWindow({
  issue,
  entries,
  defaults,
  wording,
  onClose,
}: EntryDetailsWindowProps) {
  const formId = useId();
  const [picked, setPicked] = useState<TimeEntry | undefined>();
  const { change, problem, isPending } = useChangeEntry(() => {
    setPicked(undefined);
  });
  const title = useDetailsTitle(formId, issue.key, picked);
  return (
    <EntryWindow
      title={title}
      isOpen
      onOpenChange={onClose}
      footer={
        <DetailsFooter
          formId={formId}
          picked={picked !== undefined}
          isPending={isPending}
          onClose={onClose}
          onBack={() => {
            setPicked(undefined);
          }}
        />
      }
    >
      {/* As tall as the form of one entry, so the window keeps its top edge when the form opens. */}
      <div {...partOf(part.entryDetailsBody)}>
        {picked === undefined ? (
          <EntryList entries={entries} wording={wording} onPick={setPicked} />
        ) : (
          <EntryChangeForm
            id={formId}
            entry={picked}
            issue={issue}
            defaults={defaults}
            problem={problem}
            onSubmit={(minutes, values) => {
              void change({ entry: picked, minutes, values, defaults });
            }}
          />
        )}
      </div>
    </EntryWindow>
  );
}
