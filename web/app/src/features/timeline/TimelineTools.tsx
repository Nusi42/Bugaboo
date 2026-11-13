import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  ChoiceGroup,
  Icon,
  iconName,
  iconSize,
  part,
  partOf,
  selectLook,
  Switch,
} from "@bugaboo/ui";

import { zoomWords, type ListView, type ZoomWord } from "../../shared/list-view.ts";
import type { OwnColumns } from "../issues/index.ts";
import { ViewChoices, type SearchFieldsAnswer } from "../search/index.ts";
import { DatesFrom } from "./DatesFrom.tsx";
import { timelineGroupable } from "./group-words.ts";

export interface TimelineToolsProps {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
  readonly fields: SearchFieldsAnswer;
  readonly own: OwnColumns;
  readonly onStep: (direction: -1 | 1) => void;
  readonly onToday: () => void;
  /** A phone shows the seven weeks of its list: it has no zoom and no steps through time. */
  readonly phone: boolean;
  /** *Warnings (3)*: how many are open, and whether their list is shown. */
  readonly warnings: {
    readonly count: number;
    readonly open: boolean;
    readonly onToggle: () => void;
  };
}

/** The name of each zoom: *Days · Weeks · Months · Quarters*. */
const zoomNames = {
  day: "timeline.zoom-day",
  week: "timeline.zoom-week",
  month: "timeline.zoom-month",
  quarter: "timeline.zoom-quarter",
} as const satisfies Readonly<Record<ZoomWord, string>>;

/** *Earlier*, *Today* and *Later*: the steps through time. */
function TimeSteps({ onStep, onToday }: Pick<TimelineToolsProps, "onStep" | "onToday">) {
  return (
    <>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          onStep(-1);
        }}
      >
        {t("timeline.earlier")}
      </Button>
      <Button look={buttonLook.quiet} onPress={onToday}>
        {t("timeline.today")}
      </Button>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          onStep(1);
        }}
      >
        {t("timeline.later")}
      </Button>
    </>
  );
}

/**
 * The tools over the timeline (UX §4.7): *Show as* and *Group by*, which date fields the bars use
 * (*Dates from*), *Plan by effort*, the zoom, and the steps through time with *Today*. Every choice
 * is part of the view, kept in the address and saved with the search.
 */
export function TimelineTools(props: TimelineToolsProps) {
  const { view, onView, fields, own, onStep, onToday, phone, warnings } = props;
  const { timeline } = view;
  return (
    <div {...partOf(part.timelineTools)}>
      <ViewChoices
        view={view}
        fields={fields}
        onView={onView}
        look={selectLook.inline}
        groupable={timelineGroupable(fields)}
      />
      <DatesFrom view={view} onView={onView} names={fields.names} own={own} />
      <Switch
        label={t("timeline.plan-by-effort")}
        isSelected={timeline.plan_by_effort ?? false}
        onChange={(on) => {
          onView({ ...view, timeline: { ...timeline, plan_by_effort: on } });
        }}
      />
      {phone ? null : (
        <>
          <ChoiceGroup
            legend={t("timeline.zoom")}
            choices={zoomWords.map((zoom) => ({ value: zoom, label: t(zoomNames[zoom]) }))}
            value={timeline.zoom}
            onChange={(zoom) => {
              onView({ ...view, timeline: { ...timeline, zoom } });
            }}
          />
          <TimeSteps onStep={onStep} onToday={onToday} />
        </>
      )}
      <Button look={buttonLook.quiet} aria-expanded={warnings.open} onPress={warnings.onToggle}>
        <Icon name={iconName.warning} size={iconSize.small} />
        {t("timeline.warnings", { count: warnings.count })}
      </Button>
    </div>
  );
}
