import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../errors.ts";

/** What the button needs of the mutation that makes the copy. */
export interface Creating {
  readonly isPending: boolean;
  readonly isError: boolean;
  readonly error: unknown;
  readonly mutate: () => void;
}

export interface CustomizeEditableProps {
  /** What the place uses now, in words. */
  readonly used: string;
  /** *Customize for Website*: the words of the button. */
  readonly label: string;
  readonly create: Creating;
}

/**
 * A place that has no layout or workflow of its own: what it uses now, and *Customize*, which makes
 * one as a copy of it, so the person can change it without touching the place it comes from.
 */
export function CustomizeEditable({ used, label, create }: CustomizeEditableProps) {
  return (
    <div {...partOf(part.stack)}>
      <p>{used}</p>
      <div {...partOf(part.row)}>
        <Button look={buttonLook.primary} isPending={create.isPending} onPress={create.mutate}>
          {label}
        </Button>
      </div>
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
    </div>
  );
}
