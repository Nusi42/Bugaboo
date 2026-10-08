import type { ReactNode } from "react";
import { Form as AriaForm } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface FormProps {
  /** Called when the form is sent and every field is valid. */
  readonly onSubmit: () => void;
  readonly children: ReactNode;
}

/** A form whose fields are checked when it is sent, with one problem at a time shown under its field. */
export function Form({ onSubmit, children }: FormProps) {
  return (
    <AriaForm
      {...partOf(part.stack)}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
    >
      {children}
    </AriaForm>
  );
}
