import { t } from "@bugaboo/messages";
import { Button, buttonLook } from "@bugaboo/ui";
import { lazy, Suspense, useState } from "react";

import { WaitingWindow } from "../../shared/WaitingWindow.tsx";
import { windowForms } from "../../shared/window-forms.ts";
import { AddTimeTitle } from "./add-time-title.tsx";
import type { EntryIssue } from "./entry-issue.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";

/** The window loads when it first opens, so the issue page does not carry the form. */
const AddTimeWindow = lazy(async () => ({
  default: (await import("./AddTimeWindow.tsx")).AddTimeWindow,
}));

export interface AddTimeButtonProps {
  readonly issue: EntryIssue;
}

/**
 * *Add time* beside *Time spent* on the issue page (TIME-02): the button opens the window that books
 * a length on a date. It is there only for a person who may book time on the issue, which the
 * server tells by answering what a new entry starts with.
 */
export function AddTimeButton({ issue }: AddTimeButtonProps) {
  const [open, setOpen] = useState(false);
  const defaults = useEntryDefaults(issue.key).data;
  if (defaults === undefined) {
    return null;
  }
  return (
    <>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("time.add.button")}
      </Button>
      {open ? (
        <Suspense
          fallback={
            <WaitingWindow
              form={windowForms.timeEntry}
              title={<AddTimeTitle issue={issue} />}
              onClose={() => {
                setOpen(false);
              }}
            />
          }
        >
          <AddTimeWindow
            issue={issue}
            defaults={defaults}
            onClose={() => {
              setOpen(false);
            }}
          />
        </Suspense>
      ) : null}
    </>
  );
}
