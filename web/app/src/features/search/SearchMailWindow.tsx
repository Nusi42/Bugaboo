import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Checkbox,
  part,
  partOf,
  SelectField,
  Window,
} from "@bugaboo/ui";
import { useState } from "react";

import { weekdayName, zoneNameOf } from "../../shared/clock.ts";
import { messageOf } from "../../shared/errors.ts";
import { useRegion } from "../../shared/region.ts";
import { windowLabels } from "../../shared/window-labels.ts";
import { weekOf } from "../work-times/index.ts";
import {
  firstChoice,
  hourText,
  mailHours,
  mailOf,
  useSearchMails,
  useSetSearchMail,
  type SearchMailChoice,
} from "./search-mails.ts";

export interface SearchMailWindowProps {
  /** The saved search: its ID and its name. */
  readonly search: { readonly id: string; readonly name: string };
  readonly isOpen: boolean;
  readonly onClose: () => void;
}

/** The choice the window starts from: the e-mailed search the person has of it, or the first one. */
function startingChoice(mail: SearchMailChoice | undefined): SearchMailChoice {
  return mail ?? firstChoice;
}

/** The weekdays, in the order the person's week starts, each a box to tick. */
function DayTicks({
  days,
  isDisabled,
  onChange,
}: {
  readonly days: readonly boolean[];
  readonly isDisabled: boolean;
  readonly onChange: (days: readonly boolean[]) => void;
}) {
  const week = weekOf(useRegion().firstDay);
  return (
    <fieldset {...partOf(part.searchMailDays)}>
      <legend {...partOf(part.searchMailLegend)}>{t("search.mail-days")}</legend>
      <div {...partOf(part.searchMailDayBoxes)}>
        {week.map(({ at, date }) => (
          <span key={at}>
            <Checkbox
              label={weekdayName(date, "short")}
              isSelected={days[at] === true}
              isDisabled={isDisabled}
              onChange={(ticked) => {
                onChange(days.map((each, which) => (which === at ? ticked : each)));
              }}
            />
          </span>
        ))}
      </div>
    </fieldset>
  );
}

/**
 * The form of the window: the days, the hour, *Only when something is new or changed*. It cannot be
 * used until the person's e-mailed searches are read, so a choice never overwrites one it did not
 * start from.
 */
function MailForm({
  choice,
  isDisabled,
  onChange,
}: {
  readonly choice: SearchMailChoice;
  readonly isDisabled: boolean;
  readonly onChange: (choice: SearchMailChoice) => void;
}) {
  const region = useRegion();
  const zone = zoneNameOf(region.timeZone);
  return (
    <>
      <DayTicks
        days={choice.days}
        isDisabled={isDisabled}
        onChange={(days) => {
          onChange({ ...choice, days });
        }}
      />
      <SelectField
        label={t("search.mail-hour")}
        choices={mailHours.map((hour) => ({
          value: String(hour),
          label: t("search.mail-hour-choice", { time: hourText(hour, region), zone }),
        }))}
        value={String(choice.hour)}
        isDisabled={isDisabled}
        onChange={(value) => {
          onChange({ ...choice, hour: Number(value) });
        }}
      />
      <Checkbox
        label={t("search.mail-only-when-changed")}
        isSelected={choice.only_when_changed}
        isDisabled={isDisabled}
        onChange={(only) => {
          onChange({ ...choice, only_when_changed: only });
        }}
      />
    </>
  );
}

/** *Cancel* and *E-mail me this search* at the foot of the window. */
function Footer({
  isDisabled,
  isPending,
  onCancel,
  onSave,
}: {
  readonly isDisabled: boolean;
  readonly isPending: boolean;
  readonly onCancel: () => void;
  readonly onSave: () => void;
}) {
  return (
    <div {...partOf(part.windowFooterActions)}>
      <Button onPress={onCancel}>{t("common.cancel")}</Button>
      <Button
        look={buttonLook.primary}
        isDisabled={isDisabled}
        isPending={isPending}
        onPress={onSave}
      >
        {t("search.mail-me")}
      </Button>
    </div>
  );
}

/**
 * *E-mail me this search* (QRY-07, UX design §4.2): the weekdays and the whole hour, in the person's
 * own time zone, at which the saved search comes to them by e-mail, and *Only when something is new
 * or changed*. It goes to the person only; the note under the form says what the e-mail holds, and
 * whether the organization sends it now.
 */
export function SearchMailWindow({ search, isOpen, onClose }: SearchMailWindowProps) {
  const list = useSearchMails();
  const set = useSetSearchMail();
  const [choice, setChoice] = useState<SearchMailChoice | null>(null);
  const shown = choice ?? startingChoice(mailOf(list.data, search.id));
  const noDay = !shown.days.includes(true);
  const read = list.isSuccess;
  const close = (): void => {
    setChoice(null);
    set.reset();
    onClose();
  };
  return (
    <Window
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          close();
        }
      }}
      title={t("search.mail-title", { name: search.name })}
      labels={windowLabels()}
      footer={
        <Footer
          isDisabled={noDay || !read}
          isPending={set.isPending}
          onCancel={close}
          onSave={() => {
            set.mutate({ search: search.id, choice: shown }, { onSuccess: close });
          }}
        />
      }
    >
      <div {...partOf(part.searchMail)}>
        <MailForm choice={shown} isDisabled={!read} onChange={setChoice} />
        {noDay ? <p {...partOf(part.muted)}>{t("search.mail-days-needed")}</p> : null}
        <p {...partOf(part.muted)}>
          {t("search.mail-note", { most: list.data?.most_listed ?? 0 })}
          {list.data?.short_e_mails === true ? ` ${t("search.mail-note-short")}` : null}
        </p>
        {list.data?.sending === false ? (
          <Alert tone={alertTone.warning}>{t("search.mail-not-sending")}</Alert>
        ) : null}
        {set.isError ? <Alert tone={alertTone.danger}>{messageOf(set.error)}</Alert> : null}
      </div>
    </Window>
  );
}
