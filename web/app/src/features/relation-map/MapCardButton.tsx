import { t } from "@bugaboo/messages";
import {
  buttonType,
  IssueId,
  IssueTypeIcon,
  iconSize,
  part,
  partOf,
  rowStates,
  StatusIcon,
  usePlace,
} from "@bugaboo/ui";
import { useRef } from "react";

import { drawnGroupOf, drawnPriorityOf } from "../relations/index.ts";
import type { Point } from "./geometry.ts";
import type { MapIssue } from "./layout.ts";

/** How a card or a line stands while a chain is shown: on it, or faded; nothing while none is shown. */
export type ChainMark = "on-chain" | "faded" | undefined;

/** What a card tells the picture: it is pointed at (or has the focus), left, or pressed. */
export interface Pointing {
  readonly onPoint: (id: string) => void;
  readonly onLeave: () => void;
  readonly onChoose: (id: string) => void;
}

export interface MapCardButtonProps extends Pointing {
  readonly issue: MapIssue;
  readonly place: Point;
  readonly start: boolean;
  readonly chosen: boolean;
  readonly mark: ChainMark;
}

/** The handlers of a card: the pointer and the focus show its chain, a press keeps it. */
function handlersOf(id: string, { onPoint, onLeave, onChoose }: Pointing) {
  const point = (): void => {
    onPoint(id);
  };
  return {
    onPointerEnter: point,
    onPointerLeave: onLeave,
    onFocus: point,
    onBlur: onLeave,
    onClick: () => {
      onChoose(id);
    },
  };
}

/**
 * An issue on the picture (UX §4.15): tinted by its status group and edged by its priority like a
 * row (R4), with its status icon, ID, type and title in the order of every row (R9). It is a
 * button: pointing at it or giving it the focus shows its chain, pressing it keeps the chain and
 * shows it in the side. Nothing about it moves while that happens (UX §6.4).
 */
export function MapCardButton(props: MapCardButtonProps) {
  const { issue, place, start, chosen, mark } = props;
  const card = useRef<HTMLButtonElement>(null);
  usePlace(card, { "map-x": place.x, "map-y": place.y });
  const group = drawnGroupOf(issue);
  return (
    <button
      ref={card}
      type={buttonType.button}
      data-issue={issue.readable_id}
      aria-pressed={chosen}
      {...partOf(part.mapCard, {
        states: [
          ...rowStates({ group, priority: drawnPriorityOf(issue) }),
          start && "start",
          chosen && "chosen",
          mark,
        ],
      })}
      {...handlersOf(issue.id, props)}
    >
      {start ? <span {...partOf(part.mapCardTab)}>{t("relation-map.start")}</span> : null}
      {issue.status === undefined || issue.status === null ? (
        <span />
      ) : (
        <StatusIcon group={group} size={iconSize.small} label={issue.status.name} />
      )}
      <IssueId id={issue.readable_id} />
      <IssueTypeIcon name={issue.issue_type.icon} label={issue.issue_type.name} />
      <span {...partOf(part.mapCardTitle)}>{issue.title ?? ""}</span>
    </button>
  );
}
