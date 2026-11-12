import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, partOf, part } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { AwayWindow } from "./AwayWindow.tsx";
import { useAwayChanges } from "./use-away-changes.ts";
import { awayInOrder, type Away, type WorkTimes, type Whose } from "./work-times.ts";
import { awayDaysText } from "./work-times-words.ts";

/** One away time: its days, its note for its person, and *Change* and *Delete* for who may change it. */
function AwayRow({
  away,
  mayChange,
  onChange,
  onDelete,
}: {
  readonly away: Away;
  readonly mayChange: boolean;
  readonly onChange: () => void;
  readonly onDelete: () => void;
}) {
  const days = awayDaysText(away);
  return (
    <li {...partOf(part.item, { variants: ["row"] })}>
      <span {...partOf(part.itemMain)}>
        <span {...partOf(part.itemTitle)}>{days}</span>
        {away.note === null ? null : <span {...partOf(part.muted)}>{away.note}</span>}
      </span>
      {mayChange ? (
        <span {...partOf(part.itemActions)}>
          <Button
            look={buttonLook.quiet}
            aria-label={t("work-times.away-change-name", { days })}
            onPress={onChange}
          >
            {t("work-times.away-change")}
          </Button>
          <Button
            look={buttonLook.quiet}
            aria-label={t("work-times.away-delete-name", { days })}
            onPress={onDelete}
          >
            {t("work-times.away-delete")}
          </Button>
        </span>
      ) : null}
    </li>
  );
}

/** A list of away times under its heading, or nothing when it has none. */
function AwayList({
  heading,
  times,
  render,
}: {
  readonly heading: string;
  readonly times: readonly Away[];
  readonly render: (away: Away) => ReactNode;
}) {
  return times.length === 0 ? null : (
    <section {...partOf(part.stack)} aria-label={heading}>
      <h3 {...partOf(part.cardTopic)}>{heading}</h3>
      <ul {...partOf(part.list, { variants: ["snug"] })}>{times.map((away) => render(away))}</ul>
    </section>
  );
}

/** The two lists, coming and past, with what to say when there is none or more than are shown. */
function AwayLists({
  coming,
  past,
  render,
}: {
  readonly coming: WorkTimes;
  readonly past: WorkTimes | undefined;
  readonly render: (away: Away) => ReactNode;
}) {
  const listed = awayInOrder(coming.away, past?.away ?? []);
  return (
    <>
      {listed.coming.length === 0 && listed.past.length === 0 ? (
        <p {...partOf(part.muted)}>{t("work-times.away-none")}</p>
      ) : null}
      <AwayList heading={t("work-times.away-coming")} times={listed.coming} render={render} />
      <AwayList heading={t("work-times.away-past")} times={listed.past} render={render} />
      {coming.truncated || past?.truncated === true ? (
        <p {...partOf(part.muted)}>{t("work-times.away-more")}</p>
      ) : null}
    </>
  );
}

export interface AwayTimesProps {
  readonly whose: Whose;
  readonly coming: WorkTimes;
  readonly past: WorkTimes | undefined;
  /** The person's name when the work times are somebody else's. */
  readonly otherName: string | undefined;
}

/** What the window is open for: nothing, a new away time, or the one that is changed. */
type Open = { readonly kind: "closed" } | { readonly kind: "open"; readonly changing?: Away };

/**
 * *Away times* (PLAN-11): the coming ones first, then the past year's, each as its days with *Change*
 * and *Delete*, and *Add away time*, which opens the window. An away time is deleted one year after its
 * last day; Bugaboo stores no reason, which the window says.
 */
export function AwayTimes({ whose, coming, past, otherName }: AwayTimesProps) {
  const [open, setOpen] = useState<Open>({ kind: "closed" });
  const changes = useAwayChanges(whose);
  const mayChange = coming.may_change;
  const row = (away: Away) => (
    <AwayRow
      key={away.id}
      away={away}
      mayChange={mayChange}
      onChange={() => {
        setOpen({ kind: "open", changing: away });
      }}
      onDelete={() => {
        changes.remove(away);
      }}
    />
  );
  return (
    <Card
      heading={t("work-times.away")}
      intro={t("work-times.away-intro")}
      actions={
        mayChange ? (
          <Button
            look={buttonLook.primary}
            onPress={() => {
              setOpen({ kind: "open" });
            }}
          >
            {t("work-times.away-add")}
          </Button>
        ) : undefined
      }
    >
      {changes.problem === null ? null : <Alert tone={alertTone.danger}>{changes.problem}</Alert>}
      <AwayLists coming={coming} past={past} render={row} />
      {open.kind === "open" ? (
        <AwayWindow
          changing={open.changing}
          otherName={otherName}
          onSave={(days) => changes.save(days, open.changing)}
          onClose={() => {
            setOpen({ kind: "closed" });
          }}
        />
      ) : null}
    </Card>
  );
}
