import type { EditWhat } from "@bugaboo/api-client";
import type { ReactNode } from "react";

import type { EditedThing } from "./edited-thing.ts";
import { EditLockNotices } from "./EditLockNotices.tsx";
import { useEditLock, type EditLock } from "./use-edit-lock.ts";

export interface LockedEditorProps {
  /** What the lock is about: the layout or the workflow, edited as a whole. */
  readonly what: EditWhat;
  readonly thing: EditedThing;
  /** The note that the lock was lost and the changes are kept; it comes with *Try again*. */
  readonly lost: string;
  /** The editor, given the lock. */
  readonly children: (lock: EditLock) => ReactNode;
}

/**
 * The edit lock of an editor of a layout or a workflow: taken when it opens, let go when it closes,
 * with the notes about somebody else's lock or a lost one above the editor (ADR-0019).
 */
export function LockedEditor({ what, thing, lost, children }: LockedEditorProps) {
  const lock = useEditLock(what);
  return (
    <>
      <EditLockNotices lock={lock} thing={thing} lost={lost} />
      {children(lock)}
    </>
  );
}
