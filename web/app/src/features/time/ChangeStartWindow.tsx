import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId } from "react";

import { DayBox } from "../../shared/DayBox.tsx";
import { EntryWindow } from "./EntryWindow.tsx";
import { closeTimeAsk } from "./time-asks.ts";
import { useChangeStart } from "./use-change-start.ts";

/**
 * *Change start time* (TIME-01): the day and time of day the running timer started, in the person's
 * zone. The server refuses a start in the future and one before the end of the last entry that has a
 * start, and says which.
 */
export function ChangeStartWindow() {
  const formId = useId();
  const start = useChangeStart();
  if (start === undefined) {
    return null;
  }
  const { day, setDay, time, setTime, problem, save } = start;
  return (
    <EntryWindow
      title={t("time.start-time.title")}
      isOpen
      onOpenChange={closeTimeAsk}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={closeTimeAsk}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId}>{t("time.start-time.save")}</SubmitButton>
        </div>
      }
    >
      <Form
        id={formId}
        onSubmit={() => {
          void save();
        }}
      >
        {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        <DayBox label={t("time.start-time.day")} value={day} onChange={setDay} />
        <TextField label={t("time.start-time.time")} value={time} onChange={setTime} />
      </Form>
    </EntryWindow>
  );
}
