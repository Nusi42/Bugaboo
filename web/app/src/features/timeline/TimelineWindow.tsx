import { t } from "@bugaboo/messages";
import { Button, buttonLook, Form, part, partOf, SubmitButton, Window } from "@bugaboo/ui";
import { useId, type ReactNode } from "react";

import { useWindowProps } from "../../shared/use-window-props.ts";
import type { WindowForm } from "../../shared/window-forms.ts";

export interface TimelineWindowProps {
  readonly form: WindowForm;
  readonly title: string;
  /** The name of the button that does what the window is for. */
  readonly submit: string;
  readonly onSubmit: () => void;
  readonly onClose: () => void;
  readonly children: ReactNode;
}

/** A window of the timeline (UX §6.3): its form, *Cancel* and the button that saves, at the right. */
export function TimelineWindow({
  form,
  title,
  submit,
  onSubmit,
  onClose,
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
          <SubmitButton form={formId}>{submit}</SubmitButton>
        </div>
      }
    >
      <Form id={formId} onSubmit={onSubmit}>
        {children}
      </Form>
    </Window>
  );
}
