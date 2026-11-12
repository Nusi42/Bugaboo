import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  SubmitButton,
  Window,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { useAwayDraft } from "./away-draft.ts";
import { AwayFields } from "./AwayFields.tsx";
import type { Away, AwayDays } from "./work-times.ts";

export interface AwayWindowProps {
  /** The away time that is changed; none for a new one. */
  readonly changing: Away | undefined;
  /** The person's name when the away time is somebody else's: the window says they will be told, and has no note. */
  readonly otherName: string | undefined;
  /** Saves the days; resolves to the plain message of a refusal, or `null` when it was saved. */
  readonly onSave: (days: AwayDays) => Promise<string | null>;
  readonly onClose: () => void;
}

/**
 * *Add away time* and *Change away time* (PLAN-11, UX §4.13): *From* and *To*, the first day whole or
 * afternoon only, the last day whole or morning only, and a note only its person sees. It says once
 * that Bugaboo stores no reason. For somebody else it has no note and says that they will be told. A
 * window on a computer, a full screen on a phone (R10). The server decides whether the days make an
 * away time and says why not.
 */
export function AwayWindow({ changing, otherName, onSave, onClose }: AwayWindowProps) {
  const formId = useId();
  const windowProps = useWindowProps(windowForms.awayTime);
  const draft = useAwayDraft(changing);
  const [problem, setProblem] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const own = otherName === undefined;
  const save = async (): Promise<void> => {
    setSaving(true);
    const refused = await onSave(draft.days(own));
    setSaving(false);
    setProblem(refused);
    if (refused === null) {
      onClose();
    }
  };
  return (
    <Window
      {...windowProps}
      title={t(changing === undefined ? "work-times.window-add" : "work-times.window-change")}
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isPending={saving}>
            {t(changing === undefined ? "work-times.save-add" : "work-times.save-change")}
          </SubmitButton>
        </div>
      }
    >
      <Form id={formId} onSubmit={() => void save()}>
        <p {...partOf(part.muted)}>{t("work-times.no-reason")}</p>
        {own ? null : <p {...partOf(part.text)}>{t("work-times.told", { name: otherName })}</p>}
        {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        <AwayFields draft={draft} withNote={own} />
      </Form>
    </Window>
  );
}
