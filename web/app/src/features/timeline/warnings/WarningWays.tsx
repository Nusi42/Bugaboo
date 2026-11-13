import { t } from "@bugaboo/messages";
import { Button, buttonLook, IssueId, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { formatCalendarDay, keepingHyphens } from "../../../shared/format.ts";
import type { TimelineOptions } from "../../../shared/list-view.ts";
import { daysText } from "../dates.ts";
import type { PlanWarning, TimelineRow } from "../timeline-data.ts";
import {
  keepAndAccept,
  mayGive,
  moveAsPreviewed,
  previewMoveAfter,
  type MovePreviewIssue,
} from "./ways-out.ts";

/** A way out of a warning, as the server names it. */
type Way = PlanWarning["ways_out"][number];

/** What the ways out of a warning need from the timeline. */
export interface WaysOfWarning {
  /** The row of an issue of the page, by its internal ID. */
  readonly rowOf: (issue: string) => TimelineRow | undefined;
  /** The options of the view the warnings were worked out for. */
  readonly options: TimelineOptions;
  /** *Make it longer* was confirmed: the due date changes, through the queue of the screen. */
  readonly onLonger: (row: TimelineRow, due: string) => void;
  /** *Give it to someone else…*: the usual person picker opens, with nobody suggested. */
  readonly onGive: (row: TimelineRow) => void;
  /** Saves a run of keys that waits on the bar of an issue, before a way out is sent. */
  readonly settle: (issue: string) => void;
}

/** What a way out shows before it saves anything. */
type Preview =
  | { readonly way: "move-it-after"; readonly issues: readonly MovePreviewIssue[] | undefined }
  | { readonly way: "make-it-longer" };

interface PreviewProps {
  readonly preview: Preview;
  readonly warning: PlanWarning;
  readonly row: TimelineRow;
  readonly onConfirm: () => void;
  readonly onCancel: () => void;
}

/** What the way out will change, named before it is saved, with the button that saves it and *Cancel*. */
function WayPreview({ preview, warning, row, onConfirm, onCancel }: PreviewProps) {
  const moving = preview.way === "move-it-after" ? preview.issues : undefined;
  if (preview.way === "move-it-after" && moving === undefined) {
    return null;
  }
  return (
    <div {...partOf(part.timelineWarningPreview)}>
      {moving === undefined ? (
        <p {...partOf(part.timelineWarningLine)}>
          {t("timeline.preview-longer", {
            key: keepingHyphens(row.issue.readable_id ?? ""),
            day: formatCalendarDay(warning.due_needed ?? ""),
          })}
        </p>
      ) : (
        <>
          <p {...partOf(part.timelineWarningLine)}>
            {moving.length === 0 ? t("timeline.preview-nothing") : t("timeline.preview-move")}
          </p>
          <ul {...partOf(part.timelineSideList)}>
            {moving.map((each) => (
              <li key={each.id} {...partOf(part.timelineWarningMove)}>
                <IssueId id={keepingHyphens(each.readable_id)} />
                <span>{t("timeline.preview-before", { days: daysText(each.from) })}</span>
                <span>{t("timeline.preview-after", { days: daysText(each.to) })}</span>
              </li>
            ))}
          </ul>
        </>
      )}
      <div {...partOf(part.timelineCardActions)}>
        {moving?.length === 0 ? null : (
          <Button look={buttonLook.primary} onPress={onConfirm}>
            {moving === undefined
              ? t("timeline.change-due")
              : t("timeline.move-them", { count: moving.length })}
          </Button>
        )}
        <Button look={buttonLook.quiet} onPress={onCancel}>
          {t("common.cancel")}
        </Button>
      </div>
    </div>
  );
}

export interface WarningWaysProps extends WaysOfWarning {
  readonly warning: PlanWarning;
  /** The ID of the sentence of the warning: the buttons are described by it. */
  readonly described: string;
}

/**
 * What pressing a way out does, and what confirming its preview saves. Every way first ends a run
 * of keys that waits on the bar (`settle`), so it is saved before the way is sent.
 */
function useWays(warning: PlanWarning, row: TimelineRow | undefined, ways: WaysOfWarning) {
  const [preview, setPreview] = useState<Preview | null>(null);
  const take = (way: Way): void => {
    if (row === undefined) {
      return;
    }
    ways.settle(warning.issue);
    if (way === "move-it-after") {
      setPreview({ way, issues: undefined });
      // A refused preview is said by the server's words and shows nothing.
      void previewMoveAfter(warning, row, ways.options).then((issues) => {
        setPreview(issues === undefined ? null : { way, issues });
      });
    } else if (way === "make-it-longer") {
      setPreview({ way });
    } else if (way === "give-to-someone-else") {
      ways.onGive(row);
    } else {
      void keepAndAccept(warning, row, ways.options);
    }
  };
  const confirm = (): void => {
    if (preview?.way === "move-it-after" && preview.issues !== undefined) {
      void moveAsPreviewed(preview.issues);
    } else if (
      preview?.way === "make-it-longer" &&
      row !== undefined &&
      warning.due_needed !== null
    ) {
      ways.onLonger(row, warning.due_needed);
    }
    setPreview(null);
  };
  return {
    preview,
    take,
    confirm,
    cancel: () => {
      setPreview(null);
    },
  };
}

/**
 * The ways out of one warning (UX §4.7), in the order the server offers them. *Move it after* and
 * *Make it longer* show what will change before anything is saved; *Give it to someone else…* opens
 * the usual person picker; *Keep and accept* keeps the plan as it is.
 */
export function WarningWays({ warning, described, ...ways }: WarningWaysProps) {
  const row = ways.rowOf(warning.issue);
  const { preview, take, confirm, cancel } = useWays(warning, row, ways);
  if (row === undefined) {
    return null;
  }
  return (
    <>
      <div {...partOf(part.timelineWarningWays)}>
        {warning.ways_out
          .filter((way) => way !== "give-to-someone-else" || mayGive(row))
          .map((way) => (
            <Button
              key={way}
              aria-describedby={described}
              onPress={() => {
                take(way);
              }}
            >
              {t(`plan-way-out.${way}`)}
            </Button>
          ))}
      </div>
      {preview === null ? null : (
        <WayPreview
          preview={preview}
          warning={warning}
          row={row}
          onConfirm={confirm}
          onCancel={cancel}
        />
      )}
    </>
  );
}
