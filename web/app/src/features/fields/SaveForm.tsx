import { Form, SubmitButton, partOf, part } from "@bugaboo/ui";
import { t } from "@bugaboo/messages";
import { useEffect, useRef, type ReactNode } from "react";

/**
 * Sends the value when the focus leaves the frame (not when it moves between the boxes inside it,
 * such as a day and a time).
 */
function useSendOnLeave(active: boolean, send: () => void) {
  const frame = useRef<HTMLElement>(null);
  const latest = useRef(send);
  useEffect(() => {
    latest.current = send;
  });
  useEffect(() => {
    const element = frame.current;
    const left = (): void => {
      // The focus has moved on when the timer runs: it is still inside while it goes from box to box.
      setTimeout(() => {
        if (element?.contains(document.activeElement) !== true) {
          latest.current();
        }
      }, 0);
    };
    if (active) {
      element?.addEventListener("focusout", left);
    }
    return () => {
      element?.removeEventListener("focusout", left);
    };
  }, [active]);
  return frame;
}

/**
 * The frame of the boxes of one value. Inside a form that is sent as a whole (`live`) just the
 * boxes; in a row of the Details (`inRow`) the boxes send the value when the person leaves them
 * or presses Enter, with no button; anywhere else a form with a *Save* button.
 */
export function SaveForm({
  name,
  isPending,
  live,
  inRow,
  onSubmit,
  children,
}: {
  /** The name of the field the boxes are the value of: the frame in a row is named after it ("Value of Customer"). */
  readonly name: string;
  readonly isPending: boolean;
  readonly live: boolean;
  readonly inRow: boolean;
  readonly onSubmit: () => void;
  readonly children: ReactNode;
}) {
  const frame = useSendOnLeave(inRow && !live, onSubmit);
  if (live) {
    return <div {...partOf(part.stack)}>{children}</div>;
  }
  if (inRow) {
    return (
      <section ref={frame} aria-label={t("fields.value-of", { name })}>
        <Form onSubmit={onSubmit}>{children}</Form>
      </section>
    );
  }
  return (
    <Form onSubmit={onSubmit}>
      {children}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}
