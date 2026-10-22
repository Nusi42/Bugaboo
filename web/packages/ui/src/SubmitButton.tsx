import type { ReactNode } from "react";

import { Button } from "./Button.tsx";
import { buttonLook, buttonType } from "./props.ts";

export interface SubmitButtonProps {
  readonly children: ReactNode;
  /** Fills the width of its container, as on a phone. */
  readonly wide?: boolean;
  /** Waiting for an answer: the form is not sent a second time. */
  readonly isPending?: boolean;
  /** The form cannot be sent yet; what is missing is said next to it. */
  readonly isDisabled?: boolean;
  /** The ID of the form it sends, when it stands outside it (the footer of a window). */
  readonly form?: string;
}

/** The one button of a form that does what the form is for: it sends the form. */
export function SubmitButton({
  children,
  wide = false,
  isPending = false,
  isDisabled = false,
  form,
}: SubmitButtonProps) {
  return (
    <Button
      look={buttonLook.primary}
      type={buttonType.submit}
      wide={wide}
      isPending={isPending}
      isDisabled={isDisabled}
      {...(form === undefined ? {} : { form })}
    >
      {children}
    </Button>
  );
}
