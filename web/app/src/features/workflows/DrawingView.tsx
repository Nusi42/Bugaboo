import { statusGroupWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part, StatusIcon } from "@bugaboo/ui";
import { useId, useMemo, useState } from "react";

import { setRules, type Draft, type DraftChange } from "./draft-model.ts";
import { arrowHead, arrowOf, labelBox, type Arrow } from "./drawing-layout.ts";
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

/** An arrow with the status change it stands for. */
interface Drawn extends Arrow {
  readonly change: DraftChange;
}

/** The arrows of the status changes whose two boxes are placed. */
function arrowsOf(draft: Draft, placed: Placed): readonly Drawn[] {
  return draft.changes.flatMap((change) => {
    const [from, to] = [placed[change.from], placed[change.to]];
    return from === undefined || to === undefined
      ? []
      : [{ ...arrowOf(from, to, laneOf(draft, change)), change }];
  });
}

/** The boxes: a column for each status group, with the statuses of the group in it. */
function Columns({ draft, prefix }: { readonly draft: Draft; readonly prefix: string }) {
  return (
    <div {...partOf(part.drawingColumns)}>
      {statusGroupWords.map((group) => (
        <ul key={group} aria-label={t(`status-group.${group}`)} {...partOf(part.drawingColumn)}>
          {draft.statuses
            .filter((status) => status.group === group)
            .map((status) => (
              <li
                key={status.id}
                id={boxId(prefix, status.id)}
                {...partOf(part.drawingBox, { variants: [group] })}
              >
                <StatusIcon group={group} />
                {status.name}
              </li>
            ))}
        </ul>
      ))}
    </div>
  );
}

/** The head of an arrow, drawn once and used by every line. */
function ArrowHead({ id }: { readonly id: string }) {
  return (
    <defs>
      <marker
        id={id}
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
  );
}

/** The words of an arrow as a button over it, which opens the rules of the status change. */
function ArrowLabel({
  draft,
  arrow,
  onOpen,
}: {
  readonly draft: Draft;
  readonly arrow: Drawn;
  readonly onOpen: (change: string) => void;
}) {
  const { change, middle } = arrow;
  return (
    <foreignObject
      {...partOf(part.drawingArrow)}
      x={middle.x - labelBox.width / 2}
      y={middle.y - labelBox.height / 2}
      width={labelBox.width}
      height={labelBox.height}
    >
      <Button
        look={buttonLook.quiet}
        aria-label={t("workflows.arrow-label", {
          from: nameOf(draft, change.from),
          to: nameOf(draft, change.to),
          button: change.button,
        })}
        onPress={() => {
          onOpen(change.id);
        }}
      >
        {change.button}
      </Button>
    </foreignObject>
  );
}

/** The lines of the arrows with their heads; the words of each arrow are buttons over them. */
function Arrows({
  draft,
  placed,
  onOpen,
}: {
  readonly draft: Draft;
  readonly placed: Placed;
  readonly onOpen: (change: string) => void;
}) {
  const head = useId().replaceAll(/[^a-z0-9]/giu, "");
  const markerEnd = `url(#${head})`;
  const arrows = arrowsOf(draft, placed);
  return (
    <svg {...partOf(part.drawingArrows)}>
      <ArrowHead id={head} />
      <g aria-hidden>
        {arrows.map((arrow) => (
          <path
            key={arrow.change.id}
            d={arrow.path}
            markerEnd={markerEnd}
            {...partOf(part.drawingLine)}
          />
        ))}
      </g>
      {arrows.map((arrow) => (
        <ArrowLabel key={arrow.change.id} draft={draft} arrow={arrow} onOpen={onOpen} />
      ))}
    </svg>
  );
}

/**
 * The drawing (UX §5.4, a computer only): the statuses as boxes in a column for each status group
 * and the status changes as arrows. The words of an arrow are a button that opens the rules of its
 * status change, so the keyboard reaches every arrow; everything else is done in the plain list.
 */
export function DrawingView({ editor, draft, edit }: DrawingViewProps) {
  const prefix = useId();
  const ids = useMemo(() => draft.statuses.map((status) => status.id), [draft.statuses]);
  const { frame, placed } = useBoxes(prefix, ids);
  const [open, setOpen] = useState<string | undefined>();
  const opened = draft.changes.find((each) => each.id === open);
  return (
    <div ref={frame} {...partOf(part.workflowDrawing)}>
      <Columns draft={draft} prefix={prefix} />
      <Arrows draft={draft} placed={placed} onOpen={setOpen} />
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
