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
import { MistakeNote } from "../../shared/MistakeNote.tsx";
import { useMistakes } from "../../shared/mistakes.ts";
import { EntryWindow } from "./EntryWindow.tsx";
import { changeStartTitle } from "./time-ask-titles.ts";
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
  const mistakes = useMistakes();
  if (start === undefined) {
    return null;
  }
  const { day, setDay, time, setTime, problem, save } = start;
  return (
    <EntryWindow
      title={changeStartTitle()}
      isOpen
      onOpenChange={closeTimeAsk}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={closeTimeAsk}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isDisabled={mistakes.held}>
            {t("time.start-time.save")}
          </SubmitButton>
        </div>
      }
    >
      <Form
        id={formId}
        onSubmit={() => {
          if (!mistakes.held) {
            void save();
          }
        }}
      >
        {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        <DayBox
          label={t("time.start-time.day")}
          value={day}
          onChange={setDay}
          onMistake={mistakes.mark("day")}
        />
        <TextField label={t("time.start-time.time")} value={time} onChange={setTime} />
        <MistakeNote mistakes={mistakes.list} />
      </Form>
    </EntryWindow>
  );
}
