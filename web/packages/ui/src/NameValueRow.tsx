import type { ReactNode } from "react";

import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { buttonLook } from "./props.ts";
import { SaveStatus, type SaveState, type SaveStatusProps } from "./SaveStatus.tsx";
import { part } from "./tokens.ts";

export interface NameValueListProps {
  readonly children: ReactNode;
}

/** The Details of an issue: name-and-value rows under each other. */
export function NameValueList({ children }: NameValueListProps) {
  return <dl {...partOf(part.nameValues)}>{children}</dl>;
}

export interface ChangeStatus {
  readonly state: SaveState;
  /** The words for *Saving…*, *Saved* and *Not saved yet*. */
  readonly labels: SaveStatusProps["labels"];
  readonly undoLabel: string;
  readonly onUndo: () => void;
}

export interface NameValueRowProps {
  /** The name of the field, on the left: 112 wide on a computer and 120 on a phone. */
  readonly name: string;
  /** Where the last change of the value stands. Without it the row says nothing about saving. */
  readonly status?: ChangeStatus;
  /** The value, which the person can click to change. */
  readonly children: ReactNode;
}

/**
 * A field of the Details (UX §4.4): its name on the left, its value on the right. A short value
 * saves as soon as it changes and says *Saved* with *Undo* where it happened (R7).
 */
export function NameValueRow({ name, status, children }: NameValueRowProps) {
  return (
    <div {...partOf(part.nameValue)}>
      <dt {...partOf(part.nameValueName)}>{name}</dt>
      <dd {...partOf(part.nameValueValue)}>
        {children}
        {status === undefined ? null : (
          <span {...partOf(part.nameValueSaved)}>
            <SaveStatus state={status.state} labels={status.labels} />
            {status.state === "saved" ? (
              <Button look={buttonLook.quiet} onPress={status.onUndo}>
                {status.undoLabel}
              </Button>
            ) : null}
          </span>
        )}
      </dd>
    </div>
  );
}
