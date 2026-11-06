import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { WordingNotice } from "../../shared/with-wording.tsx";
import { useOthersTrail } from "./others-trail.ts";
import { OtherWeek } from "./OtherWeek.tsx";
import { PeoplePicker } from "./PeoplePicker.tsx";
import { useThisWeek } from "./use-week.ts";

export interface OthersPageProps {
  /** The ID of the person whose week is open, from the address; none while nobody is chosen. */
  readonly person: string | undefined;
  /** The first date of the week, from the address; the week of today without it. */
  readonly week: string | undefined;
  readonly onWeek: (first: string) => void;
  readonly onPerson: (person: string) => void;
}

/**
 * *Working hours of others* (TIME-12, PRIV-09, ADR-0035, decision 8): one other person's week in
 * the work-day rows, picked by name, looked at until *Change*, and only their week total while
 * *Totals only* is on. It shows no time on issues, and no list puts people in an order by their hours.
 */
export function OthersPage({ person, week, onWeek, onPerson }: OthersPageProps) {
  usePageTitle(t("time.others.title"));
  const thisWeek = useThisWeek();
  const first = week ?? thisWeek;
  useOthersTrail();
  return (
    <Card heading={t("time.others.title")} level={1}>
      <div {...partOf(part.sheet)}>
        <WordingNotice />
        {person === undefined ? (
          <>
            <p {...partOf(part.muted)}>{t("time.others.choose")}</p>
            <PeoplePicker onPick={onPerson} />
          </>
        ) : (
          <OtherWeek person={person} first={first} onWeek={onWeek} onPerson={onPerson} />
        )}
      </div>
    </Card>
  );
}
