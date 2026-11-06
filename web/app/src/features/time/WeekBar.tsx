import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  IconButton,
  iconName,
  SaveStatus,
  type SaveState,
  partOf,
  part,
} from "@bugaboo/ui";

import { formatCalendarDay } from "../../shared/format.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import type { Region } from "../../shared/region.ts";
import { addDays } from "../../shared/format.ts";

export interface WeekBarProps {
  /** The first date of the week shown. */
  readonly first: string;
  /** The first date of the week of today, which *This week* goes to. */
  readonly current: string;
  readonly region: Region;
  /** The state of saving; nothing is saved on a page that only reads. */
  readonly state?: SaveState;
  readonly onWeek: (first: string) => void;
}

const DAYS_PER_WEEK = 7;

/** The week picker: the week before and after as arrows around the week, *This week* and the state of saving. */
export function WeekBar({ first, current, region, state = "idle", onWeek }: WeekBarProps) {
  return (
    <div {...partOf(part.sheetBar)}>
      <IconButton
        icon={iconName.back}
        label={t("time.sheet.previous-week")}
        onPress={() => {
          onWeek(addDays(first, -DAYS_PER_WEEK));
        }}
      />
      <span {...partOf(part.sheetWeek)}>
        {t("time.sheet.week-of", { date: formatCalendarDay(first, region) })}
      </span>
      <IconButton
        icon={iconName.next}
        label={t("time.sheet.next-week")}
        onPress={() => {
          onWeek(addDays(first, DAYS_PER_WEEK));
        }}
      />
      <span {...partOf(part.sheetBarAction)}>
        <Button
          look={buttonLook.quiet}
          isDisabled={first === current}
          onPress={() => {
            onWeek(current);
          }}
        >
          {t("time.sheet.this-week")}
        </Button>
      </span>
      <SaveStatus state={state} labels={saveLabels()} />
    </div>
  );
}
