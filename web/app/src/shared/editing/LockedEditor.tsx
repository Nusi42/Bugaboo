import type { EditWhat } from "@bugaboo/api-client";
import { useState, type ReactNode } from "react";

import type { EditedThing } from "./edited-thing.ts";
import { EditLockNotices } from "./EditLockNotices.tsx";
import { useEditLock, type EditLock } from "./use-edit-lock.ts";

/** What the editor is given besides the lock: whether what is on screen was just saved, and how to say so. */
export interface Saving {
  /** The version on screen is the one the last save made. */
  readonly saved: boolean;
  /** A save worked; says the version it made. */
  readonly onSaved: (version: number | undefined) => void;
}

export interface LockedEditorProps {
  /** What the lock is about: the layout or the workflow, edited as a whole. */
  readonly what: EditWhat;
  readonly thing: EditedThing;
  /** The note that the lock was lost and the changes are kept; it comes with *Try again*. */
  readonly lost: string;
  /** The version on screen: a save changes it, and the editor starts again from what was saved. */
  readonly version: number | undefined;
  /** The editor, given the lock. */
  readonly children: (lock: EditLock, saving: Saving) => ReactNode;
}

/**
 * The edit lock of an editor of a layout or a workflow: taken when it opens, let go when it closes,
 * with the notes about somebody else's lock or a lost one above the editor (ADR-0019). A save
 * changes the version, so the editor starts again from what was saved (the caller keys it on the
 * version) and remembers that this version is the one that was saved; the lock is not given up, the
 * screen asks for it again.
 */
export function LockedEditor({ what, thing, lost, version, children }: LockedEditorProps) {
  const lock = useEditLock(what);
  const [savedAt, setSavedAt] = useState<number | undefined>();
  return (
    <>
      <EditLockNotices lock={lock} thing={thing} lost={lost} />
      {children(lock, { saved: savedAt !== undefined && savedAt === version, onSaved: setSavedAt })}
    </>
  );
}
