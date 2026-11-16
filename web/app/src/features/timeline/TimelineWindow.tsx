import { t } from "@bugaboo/messages";
import { Button, buttonLook, Form, part, partOf, SubmitButton, Window } from "@bugaboo/ui";
import { useId, type ReactNode } from "react";

import { MistakeNote } from "../../shared/MistakeNote.tsx";
import type { Mistakes } from "../../shared/mistakes.ts";
import { useWindowProps } from "../../shared/use-window-props.ts";
import type { WindowForm } from "../../shared/window-forms.ts";

export interface TimelineWindowProps {
  readonly form: WindowForm;
  readonly title: string;
  /** The name of the button that does what the window is for. */
  readonly submit: string;
  readonly onSubmit: () => void;
  readonly onClose: () => void;
  /** A box holds no value yet: the button that saves is off and the window says why. */
  readonly mistakes?: Mistakes;
  readonly children: ReactNode;
}

/** No box holds text that is no value. */
const noMistakes: Mistakes = [];

/** A window of the timeline (UX §6.3): its form, *Cancel* and the button that saves, at the right. */
export function TimelineWindow({
  form,
  title,
  submit,
  onSubmit,
  onClose,
  mistakes = noMistakes,
  children,
}: TimelineWindowProps) {
  const formId = useId();
  return (
    <Window
      {...useWindowProps(form)}
      title={title}
      isOpen
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isDisabled={mistakes.length > 0}>
            {submit}
          </SubmitButton>
        </div>
      }
    >
      <Form
        id={formId}
        onSubmit={() => {
          if (mistakes.length === 0) {
            onSubmit();
          }
        }}
      >
        {children}
        <MistakeNote mistakes={mistakes} />
      </Form>
    </Window>
  );
}
