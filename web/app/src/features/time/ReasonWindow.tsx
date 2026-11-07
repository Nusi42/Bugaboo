import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  SubmitButton,
  TextField,
  Window,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { weekdayAndDate } from "../../shared/clock.ts";
import type { Region } from "../../shared/region.ts";
import { windowLabels } from "../../shared/window-labels.ts";
import { IdsIn } from "./IdsIn.tsx";
import type { Change } from "./use-day-cells.ts";
import type { SheetEditing } from "./use-sheet-editing.ts";

export interface ReasonWindowProps {
  readonly editing: SheetEditing;
  readonly region: Region;
  /** Whose day it is when it is somebody else's: they are told with the reason. */
  readonly other: string | undefined;
}

/** What the person typed, what the server answered, and whether the change is on its way. */
function useReason(editing: SheetEditing) {
  const [reason, setReason] = useState("");
  const [problem, setProblem] = useState<string | undefined>();
  const [saving, setSaving] = useState(false);
  const close = (): void => {
    setReason("");
    setProblem(undefined);
    editing.cancelReason();
  };
  const send = async (): Promise<void> => {
    setSaving(true);
    const refused = await editing.confirmReason(reason.trim());
    setSaving(false);
    setProblem(refused);
    if (refused === undefined) {
      setReason("");
    }
  };
  return { reason, setReason, problem, saving, close, send };
}

/** What changes and that the old time stays in the history; the breaks say only the second. */
function WhatChanges({ what }: { readonly what: Change | undefined }) {
  if (what === undefined) {
    return <p {...partOf(part.sheetChange)}>{t("time.sheet.reason-change-breaks")}</p>;
  }
  const words = { before: what.before, after: what.after };
  const isStart = what.field === "start";
  const key =
    what.before === ""
      ? isStart
        ? "time.sheet.reason-new-start"
        : "time.sheet.reason-new-end"
      : isStart
        ? "time.sheet.reason-change-start"
        : "time.sheet.reason-change-end";
  return (
    <p {...partOf(part.sheetChange)}>
      <IdsIn
        sentence={t(key, words)}
        lengths={[what.before, what.after].filter((each) => each !== "")}
      />
    </p>
  );
}

/**
 * The reason a change of a past day, or of the day of somebody else, asks for before it is saved
 * (TIME-10, ADR-0035, decision 7). The window says why a reason is needed and that the old value
 * stays in the history of the day; for another person it also says that they will be told.
 */
export function ReasonWindow({ editing, region, other }: ReasonWindowProps) {
  const formId = useId();
  const form = useReason(editing);
  if (editing.pending === undefined) {
    return null;
  }
  const date = editing.pending.date;
  const day = weekdayAndDate(date, region);
  return (
    <Window
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          form.close();
        }
      }}
      title={t("time.sheet.reason-title", { day })}
      labels={windowLabels()}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={form.close}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isPending={form.saving}>
            {t("time.sheet.reason-save")}
          </SubmitButton>
        </div>
      }
    >
      <Form id={formId} onSubmit={() => void form.send()}>
        <TextField
          label={t("time.sheet.reason-label")}
          requiredMessage={t("time.sheet.reason-required")}
          value={form.reason}
          onChange={form.setReason}
        />
        <WhatChanges what={editing.pending.what} />
        <p {...partOf(part.muted)}>
          {other === undefined
            ? t("time.sheet.reason-own-past", { day })
            : t("time.sheet.reason-other", { person: other })}
        </p>
        {form.problem === undefined ? null : <Alert tone={alertTone.danger}>{form.problem}</Alert>}
      </Form>
    </Window>
  );
}
