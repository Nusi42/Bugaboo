import { statusGroupWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part, StatusIcon } from "@bugaboo/ui";
import { useId, useMemo, useState } from "react";

import {
  changesFrom,
  setRules,
  type Draft,
  type DraftChange,
  type DraftStatus,
} from "./draft-model.ts";
import { arrowHead, arrowOf, type Arrow } from "./drawing-layout.ts";
import { boxId, useBoxes, type Placed } from "./use-boxes.ts";
import { RulesWindow } from "./RulesWindow.tsx";
import type { Edit } from "./use-draft-workflow.ts";
import type { WorkflowEditor } from "./workflows.ts";

export interface DrawingViewProps {
  readonly editor: WorkflowEditor;
  readonly draft: Draft;
  readonly edit: Edit;
}

/** The name of a status of the draft by its ID in the draft. */
function nameOf(draft: Draft, id: string): string {
  return draft.statuses.find((each) => each.id === id)?.name ?? id;
}

/** The lane an arrow runs in: a way back between the same two boxes runs on the other side of the way there. */
function laneOf(draft: Draft, change: DraftChange): number {
  const back = draft.changes.some((each) => each.from === change.to && each.to === change.from);
  if (!back) {
    return 0;
  }
  return change.from < change.to ? 1 : -1;
}

/** The arrows of the status changes whose two boxes are placed, each with its ID. */
function arrowsOf(draft: Draft, placed: Placed): readonly (Arrow & { readonly id: string })[] {
  return draft.changes.flatMap((change) => {
    const [from, to] = [placed[change.from], placed[change.to]];
    return from === undefined || to === undefined
      ? []
      : [{ ...arrowOf(from, to, laneOf(draft, change)), id: change.id }];
  });
}

/** One status as a box: its name with the icon of its group, and a button for each way out of it. */
function Box({
  draft,
  status,
  prefix,
  onOpen,
}: {
  readonly draft: Draft;
  readonly status: DraftStatus;
  readonly prefix: string;
  readonly onOpen: (change: string) => void;
}) {
  return (
    <li id={boxId(prefix, status.id)} {...partOf(part.drawingBox, { variants: [status.group] })}>
      <span {...partOf(part.drawingBoxName)}>
        <StatusIcon group={status.group} />
        {status.name}
      </span>
      <ul {...partOf(part.drawingBoxChanges)}>
        {changesFrom(draft, status.id).map((change) => (
          <li key={change.id}>
            <Button
              look={buttonLook.quiet}
              aria-label={t("workflows.arrow-label", {
                from: status.name,
                to: nameOf(draft, change.to),
                button: change.button,
              })}
              onPress={() => {
                onOpen(change.id);
              }}
            >
              {change.button}
            </Button>
          </li>
        ))}
      </ul>
    </li>
  );
}

/** The boxes: a column for each status group, with the statuses of the group in it. */
function Columns({
  draft,
  prefix,
  onOpen,
}: {
  readonly draft: Draft;
  readonly prefix: string;
  readonly onOpen: (change: string) => void;
}) {
  return (
    <div {...partOf(part.drawingColumns)}>
      {statusGroupWords.map((group) => (
        <ul key={group} aria-label={t(`status-group.${group}`)} {...partOf(part.drawingColumn)}>
          {draft.statuses
            .filter((status) => status.group === group)
            .map((status) => (
              <Box key={status.id} draft={draft} status={status} prefix={prefix} onOpen={onOpen} />
            ))}
        </ul>
      ))}
    </div>
  );
}

/** The lines of the arrows with their heads, drawn behind the boxes. */
function Lines({ arrows }: { readonly arrows: readonly (Arrow & { readonly id: string })[] }) {
  const head = useId().replaceAll(/[^a-z0-9]/giu, "");
  const markerEnd = `url(#${head})`;
  return (
    <svg aria-hidden {...partOf(part.drawingArrows)}>
      <defs>
        <marker
          id={head}
          viewBox={arrowHead.viewBox}
          refX={arrowHead.refX}
          refY={arrowHead.refY}
          markerWidth={arrowHead.markerWidth}
          markerHeight={arrowHead.markerHeight}
          orient={arrowHead.orient}
        >
          <path {...partOf(part.drawingHead)} d={arrowHead.path} />
        </marker>
      </defs>
      {arrows.map((arrow) => (
        <path key={arrow.id} d={arrow.path} markerEnd={markerEnd} {...partOf(part.drawingLine)} />
      ))}
    </svg>
  );
}

/**
 * The drawing (UX §5.4, a computer only): the statuses as boxes in a column for each status group
 * and the status changes as arrows between them. The words of each way out of a box are a button
 * in the box that opens the rules of that status change, so the keyboard reaches every one;
 * everything else is done in the plain list.
 */
export function DrawingView({ editor, draft, edit }: DrawingViewProps) {
  const prefix = useId();
  const ids = useMemo(() => draft.statuses.map((status) => status.id), [draft.statuses]);
  const { frame, placed } = useBoxes(prefix, ids);
  const [open, setOpen] = useState<string | undefined>();
  const opened = draft.changes.find((each) => each.id === open);
  return (
    <div ref={frame} {...partOf(part.workflowDrawing)}>
      <Lines arrows={arrowsOf(draft, placed)} />
      <Columns draft={draft} prefix={prefix} onOpen={setOpen} />
      {opened === undefined ? null : (
        <RulesWindow
          isOpen
          onOpenChange={(isOpen) => {
            setOpen(isOpen ? opened.id : undefined);
          }}
          editor={editor}
          change={opened}
          from={nameOf(draft, opened.from)}
          to={nameOf(draft, opened.to)}
          onChange={(rules) => {
            edit((before) => setRules(before, opened.id, rules));
          }}
        />
      )}
    </div>
  );
}
