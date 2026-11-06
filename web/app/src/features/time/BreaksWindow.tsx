import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  fieldKind,
  Form,
  SubmitButton,
  TextField,
  Window,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { windowLabels } from "../../shared/window-labels.ts";
import { breaksAsLines, EXAMPLE_BREAK } from "./breaks-lines.ts";
import { IdsIn } from "./IdsIn.tsx";
import type { WorkDay } from "./sheet-model.ts";
import type { DaysProps } from "./sheet-props.ts";
import { useDayTitle } from "./use-day-title.ts";
import { useSaveBreaks } from "./use-save-breaks.ts";

export interface BreaksWindowProps {
  readonly date: string;
  readonly day: WorkDay | undefined;
  readonly region: DaysProps["region"];
  readonly editing: DaysProps["editing"];
  readonly onClose: () => void;
}

/**
 * The breaks of one day (TIME-08): a line for each, like `12:10-12:40`. They are read and saved as
 * the cell *Breaks* reads and saves them (`readBreaks`, `planWorkDayEdit`), with the same reason
 * for a past day, so a day with several breaks is looked at and changed in one place.
 */
export function BreaksWindow({ date, day, region, editing, onClose }: BreaksWindowProps) {
  const formId = useId();
  const [lines, setLines] = useState(() => breaksAsLines(day, region));
  const { save, problem, saving } = useSaveBreaks(date, editing, onClose);
  return (
    <Window
      isOpen
      onOpenChange={onClose}
      title={t("time.breaks.title", { day: useDayTitle(date) })}
      labels={windowLabels()}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isPending={saving}>
            {t("time.breaks.save")}
          </SubmitButton>
        </div>
      }
    >
      <Form
        id={formId}
        onSubmit={() => {
          void save(lines);
        }}
      >
        {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        <span {...partOf(part.lengthBox)}>
          <TextField
            label={t("time.breaks.label")}
            kind={fieldKind.multiline}
            value={lines}
            onChange={setLines}
          />
        </span>
        <p {...partOf(part.muted)}>
          <IdsIn
            sentence={t("time.breaks.hint", { example: EXAMPLE_BREAK })}
            lengths={[EXAMPLE_BREAK]}
          />
        </p>
      </Form>
    </Window>
  );
}
