import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { personName } from "../../shared/person.ts";
import { NotFoundPage } from "../shell/index.ts";
import { PeoplePicker } from "./PeoplePicker.tsx";
import { OtherSheet } from "./Sheet.tsx";
import { TotalsOnly } from "./TotalsOnly.tsx";
import { useWorkDays } from "./use-week.ts";

export interface OtherWeekProps {
  /** The ID of the person whose week is open. */
  readonly person: string;
  /** The first date of the week. */
  readonly first: string;
  readonly onWeek: (first: string) => void;
  readonly onPerson: (person: string) => void;
}

interface WhoBarProps {
  readonly name: string;
  readonly mayChange: boolean;
  readonly changing: boolean;
  readonly onChoose: () => void;
  readonly onChange: () => void;
}

/** Who the week is of, *Choose another person* and, for a holder of the permission, *Change*. */
function WhoBar({ name, mayChange, changing, onChoose, onChange }: WhoBarProps) {
  return (
    <div {...partOf(part.sheetWho)}>
      <span>{t("time.others.person-is", { person: name })}</span>
      <Button look={buttonLook.quiet} onPress={onChoose}>
        {t("time.others.other-person")}
      </Button>
      {mayChange ? (
        <Button onPress={onChange}>
          {changing ? t("time.others.done") : t("time.others.change")}
        </Button>
      ) : null}
    </div>
  );
}

/**
 * The week of one other person: who it is, whether the reader may change it (*Change* turns the
 * cells on, and every change then asks for a reason), and the work-day rows; only the total of the
 * week while *Totals only* is on.
 */
export function OtherWeek({ person, first, onWeek, onPerson }: OtherWeekProps) {
  const days = useWorkDays(first, person);
  const page = days.data;
  const [choosing, setChoosing] = useState(false);
  const [changing, setChanging] = useState(false);
  if (days.error !== null && page === undefined) {
    return <NotFoundPage />;
  }
  const name = page === undefined ? "" : personName(page.person);
  return (
    <>
      <WhoBar
        name={name}
        mayChange={page?.may_change === true && !page.totals_only}
        changing={changing}
        onChoose={() => {
          setChoosing(!choosing);
        }}
        onChange={() => {
          setChanging(!changing);
        }}
      />
      {choosing ? <PeoplePicker onPick={onPerson} /> : null}
      {page?.totals_only === true ? (
        <TotalsOnly page={page} first={first} name={name} onWeek={onWeek} />
      ) : (
        <>
          <p {...partOf(part.muted)}>
            {changing
              ? t("time.others.changing", { person: name })
              : t("time.others.reading", { person: name })}
          </p>
          <OtherSheet first={first} onWeek={onWeek} person={person} changing={changing} />
        </>
      )}
    </>
  );
}
