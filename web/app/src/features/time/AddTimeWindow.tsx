import { t } from "@bugaboo/messages";
import { Button, buttonLook, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { AddTimeForm } from "./AddTimeForm.tsx";
import type { EntryIssue } from "./entry-issue.ts";
import { EntryWindow } from "./EntryWindow.tsx";
import { AddTimeTitle } from "./add-time-title.tsx";
import { useAddTime } from "./use-add-time.ts";
import { useAddTimeState } from "./use-add-time-state.ts";
import type { EntryDefaults } from "./use-entry-defaults.ts";

export interface AddTimeWindowProps {
  readonly issue: EntryIssue;
  readonly defaults: EntryDefaults;
  readonly onClose: () => void;
}

/**
 * *Add time* (TIME-02): a window to book a length on a date by hand. The date starts as the
 * person's today; *Activity* and *Cost center* start with the defaults of the issue.
 */
export function AddTimeWindow({ issue, defaults, onClose }: AddTimeWindowProps) {
  const formId = useId();
  const state = useAddTimeState(defaults);
  const { add, problem, isPending } = useAddTime(onClose);
  const { date, minutes, values, ready } = state;
  return (
    <EntryWindow
      title={<AddTimeTitle issue={issue} />}
      isOpen
      onOpenChange={onClose}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isPending={isPending} isDisabled={!ready}>
            {t("time.add.save")}
          </SubmitButton>
        </div>
      }
    >
      <AddTimeForm
        id={formId}
        issue={issue}
        defaults={defaults}
        state={state}
        problem={problem}
        onSubmit={() => {
          if (ready && minutes !== null) {
            void add({ issue: issue.key, date, minutes, values, defaults });
          }
        }}
      />
    </EntryWindow>
  );
}
