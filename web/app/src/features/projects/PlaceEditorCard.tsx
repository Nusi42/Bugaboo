import { Card, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { QueryGate } from "../shell/index.ts";

export interface PlaceEditorCardProps<Editor> {
  /** The heading of the card: the place, in words. */
  readonly heading: string;
  /** What the editor is for, in a sentence. */
  readonly intro: string;
  /** Chooses what is edited: the issue type, the form. */
  readonly chooser: ReactNode;
  /** More sentences between the choice and the editor. */
  readonly notes?: ReactNode;
  /** What the read of the editor came to. */
  readonly error: unknown;
  readonly data: Editor | undefined;
  /** The editor, or the way to make one, once it is read. */
  readonly children: (editor: Editor) => ReactNode;
}

/**
 * The card of an editor that belongs to a place (a layout, a workflow): the heading, what it is for,
 * the choice of what is edited, and the editor itself once it is read. The same card serves a
 * project and the whole organization, and the layouts and the workflows.
 */
export function PlaceEditorCard<Editor>({
  heading,
  intro,
  chooser,
  notes,
  error,
  data,
  children,
}: PlaceEditorCardProps<Editor>) {
  return (
    <Card heading={heading} level={1}>
      <div {...partOf(part.stack)}>
        <p {...partOf(part.muted)}>{intro}</p>
        {chooser}
        {notes}
        <QueryGate error={error} data={data}>
          {children}
        </QueryGate>
      </div>
    </Card>
  );
}
