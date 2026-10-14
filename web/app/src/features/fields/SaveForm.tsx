import { Form, SubmitButton, partOf, part } from "@bugaboo/ui";
import { t } from "@bugaboo/messages";
import type { ReactNode } from "react";

/** A form with a save button around the boxes of one value; inside a form that is sent as a whole, just the boxes. */
export function SaveForm({
  isPending,
  live,
  onSubmit,
  children,
}: {
  readonly isPending: boolean;
  readonly live: boolean;
  readonly onSubmit: () => void;
  readonly children: ReactNode;
}) {
  if (live) {
    return <div {...partOf(part.stack)}>{children}</div>;
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
