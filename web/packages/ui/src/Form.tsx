import { useEffect, useRef, type ReactNode } from "react";
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

/** The boxes in which Enter sends the form, as a browser does where the button stands in the form. */
const SENT_BY_ENTER = new Set(["text", "search", "email", "url", "tel", "password", "number"]);

/**
 * Whether Enter in this box should send the form itself: a box that a browser sends a form from,
 * in a form whose button stands outside it (`form` attribute, the footer of a window). Chromium does
 * that by itself; WebKit does not, so the form does it, the same way in every browser: the fields are
 * checked and the form is sent as by its button (M8-F126).
 */
function sendsOnEnter(box: unknown, buttonInside: boolean): boolean {
  return box instanceof HTMLInputElement && SENT_BY_ENTER.has(box.type) && !buttonInside;
}

/** A form whose fields are checked when it is sent, with one problem at a time shown under its field. */
export function Form({ onSubmit, id, children }: FormProps) {
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    const element = form.current;
    const onKey = (event: KeyboardEvent): void => {
      const enter = event.key === "Enter" && !event.isComposing && !event.defaultPrevented;
      const inside = element?.querySelector("[type=submit]") !== null;
      if (element !== null && enter && sendsOnEnter(event.target, inside)) {
        event.preventDefault();
        element.requestSubmit();
      }
    };
    element?.addEventListener("keydown", onKey);
    return () => {
      element?.removeEventListener("keydown", onKey);
    };
  }, []);
  return (
    <AriaForm
      {...partOf(part.stack)}
      {...(id === undefined ? {} : { id })}
      ref={form}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
    >
      {children}
    </AriaForm>
  );
}
