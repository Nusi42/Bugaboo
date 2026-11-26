import { t } from "@bugaboo/messages";
import { Button, buttonLook, Form, part, partOf, SubmitButton, Window } from "@bugaboo/ui";
import { useId, type ReactNode } from "react";

import { MistakeNote } from "./MistakeNote.tsx";
import type { Mistakes } from "./mistakes.ts";
import { useWindowProps } from "./use-window-props.ts";
import type { WindowForm } from "./window-forms.ts";

export interface FormWindowProps {
  readonly form: WindowForm;
  readonly title: string;
  /** The name of the button that does what the window is for. */
  readonly submit: string;
  readonly onSubmit: () => void;
  readonly onClose: () => void;
  /** A box holds no value yet: the button that saves is off and the window says why. */
  readonly mistakes?: Mistakes;
  /** More buttons of the footer, before *Cancel*: a way out that is not a save (*Delete*). */
  readonly extra?: ReactNode;
  readonly children: ReactNode;
}

/** No box holds text that is no value. */
const noMistakes: Mistakes = [];

/**
 * A window with one form (UX §6.3): its form, *Cancel* and the button that saves, at the right; `extra`
 * stands before them, at the start of the footer. The window of *Set dates…*, of *Change due date …* and
 * of a release.
 */
export function FormWindow({
  form,
  title,
  submit,
  onSubmit,
  onClose,
  mistakes = noMistakes,
  extra,
  children,
}: FormWindowProps) {
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
          {extra}
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
