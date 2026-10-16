import type { ReactNode } from "react";
import { Form as AriaForm } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface FormProps {
  /** Called when the form is sent and every field is valid. */
  readonly onSubmit: () => void;
  /** Names the form, for a submit button that stands outside it (the footer of a window). */
  readonly id?: string;
  readonly children: ReactNode;
}

/** A form whose fields are checked when it is sent, with one problem at a time shown under its field. */
export function Form({ onSubmit, id, children }: FormProps) {
  return (
    <AriaForm
      {...partOf(part.stack)}
      {...(id === undefined ? {} : { id })}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
    >
      {children}
    </AriaForm>
  );
}
