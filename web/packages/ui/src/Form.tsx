import type { ReactNode } from "react";
import { Form as AriaForm } from "react-aria-components";

import styles from "./Form.module.css";
import { cx } from "./cx.ts";

export interface FormProps {
  /** Called when the form is sent and every field is valid. */
  readonly onSubmit: () => void;
  readonly children: ReactNode;
}

/** A form whose fields are checked when it is sent, with one problem at a time shown under its field. */
export function Form({ onSubmit, children }: FormProps) {
  return (
    <AriaForm
      className={cx(styles.form)}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
    >
      {children}
    </AriaForm>
  );
}
