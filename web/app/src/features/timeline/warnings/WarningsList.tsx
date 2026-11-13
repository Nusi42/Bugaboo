import { t } from "@bugaboo/messages";
import { buttonType, Icon, iconName, iconSize, part, partOf } from "@bugaboo/ui";
import { useId } from "react";

import { personName } from "../../../shared/person.ts";
import { warningKey, type PlanWarning } from "../timeline-data.ts";
import { WarningWays, type WaysOfWarning } from "./WarningWays.tsx";
import { warningSentence } from "./warning-words.ts";

export interface WarningsListProps extends WaysOfWarning {
  readonly warnings: readonly PlanWarning[];
  /** A warning is pointed at or has the focus: its bar is marked; none when it is left. */
  readonly onMark: (issue?: string) => void;
  /** The sentence of a warning was pressed: its bar is selected. */
  readonly onShow: (issue: string) => void;
}

/**
 * The sentence that ends the list (UX §4.7): about away times where the list has *Away*, and about
 * days without work where it has only the neutral warning of a person the reader does not see, so it
 * names no absence there.
 */
function closingOf(warnings: readonly PlanWarning[]): string {
  const neutral =
    warnings.some((warning) => warning.kind === "days-without-work") &&
    !warnings.some((warning) => warning.kind === "away");
  return neutral ? t("timeline.warnings-closing-neutral") : t("timeline.warnings-closing");
}

/** The warnings in the order of the list: the open ones, then the accepted ones, faded, at the end. */
function inListOrder(warnings: readonly PlanWarning[]): readonly PlanWarning[] {
  return [
    ...warnings.filter((warning) => warning.accepted === null),
    ...warnings.filter((warning) => warning.accepted !== null),
  ];
}

interface WarningItemProps extends Omit<WarningsListProps, "warnings"> {
  readonly warning: PlanWarning;
}

/** One warning: its sign and sentence, which marks and selects its bar, and its ways out or who accepted it. */
function WarningItem({ warning, onMark, onShow, ...ways }: WarningItemProps) {
  const described = useId();
  const row = ways.rowOf(warning.issue);
  const sentence = warningSentence(warning, {
    key: row?.issue.readable_id ?? "",
    project: row?.issue.project?.name ?? "",
  });
  const accepted = warning.accepted;
  const look = partOf(part.timelineWarning, { states: [accepted !== null && "accepted"] });
  const mark = (): void => {
    onMark(warning.issue);
  };
  const leave = (): void => {
    onMark();
  };
  return (
    <li {...look}>
      <button
        type={buttonType.button}
        {...partOf(part.timelineWarningSentence)}
        onClick={() => {
          onShow(warning.issue);
        }}
        onPointerEnter={mark}
        onPointerLeave={leave}
        onFocus={mark}
        onBlur={leave}
      >
        <span {...partOf(part.timelineWarningSign)}>
          <Icon name={iconName.warning} size={iconSize.small} />
        </span>
        <span id={described}>{sentence}</span>
      </button>
      {accepted === null ? (
        <WarningWays warning={warning} described={described} {...ways} />
      ) : (
        <p {...partOf(part.muted)}>
          {t("timeline.warning-accepted", { name: personName(accepted.by) })}
        </p>
      )}
    </li>
  );
}

/**
 * The list of plan warnings (UX §4.7, PLAN-13): each with its sign, the one sentence the server
 * picked, and its ways out; an accepted warning faded at the end with who accepted it, and the
 * closing sentence. Pointing at a warning marks its bar.
 */
export function WarningsList({ warnings, ...item }: WarningsListProps) {
  return (
    <section {...partOf(part.timelineWarnings)} aria-label={t("timeline.warnings-title")}>
      <h2 {...partOf(part.timelineSideHeading)}>{t("timeline.warnings-title")}</h2>
      {warnings.length === 0 ? (
        <p {...partOf(part.muted)}>{t("timeline.warnings-none")}</p>
      ) : (
        <ul {...partOf(part.timelineSideList)}>
          {inListOrder(warnings).map((warning) => (
            <WarningItem key={warningKey(warning)} warning={warning} {...item} />
          ))}
        </ul>
      )}
      <p {...partOf(part.muted)}>{closingOf(warnings)}</p>
    </section>
  );
}
