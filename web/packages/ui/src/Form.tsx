import { useEffect, useRef, type ReactNode } from "react";
import { Form as AriaForm } from "react-aria-components";

import { partOf } from "./parts.ts";
import { plainEnter } from "./plain-enter.ts";
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
 * Sends a form on Enter in a box when its button stands outside it (`form` attribute, the footer of a
 * window). Chromium does that by itself; WebKit does not, so the form does it, the same way in every
 * browser: the fields are checked and the form is sent as by its button (M8-F126). It listens on the
 * document, after React has handed the key to the parts of the form, so a part that takes Enter for
 * itself (and prevents the default) wins. A field that stops the key on its way (a list that is open)
 * keeps it from the form as well, so that field alone decides what Enter does there.
 */
function useSendOnEnter() {
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      const element = form.current;
      const box = event.target;
      const sends =
        element !== null &&
        box instanceof HTMLInputElement &&
        element.contains(box) &&
        SENT_BY_ENTER.has(box.type) &&
        element.querySelector("[type=submit]") === null;
      if (sends && plainEnter(event)) {
        event.preventDefault();
        element.requestSubmit();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
    };
  }, []);
  return form;
}

/** A form whose fields are checked when it is sent, with one problem at a time shown under its field. */
export function Form({ onSubmit, id, children }: FormProps) {
  const form = useSendOnEnter();
  return (
    <AriaForm
      ref={form}
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
