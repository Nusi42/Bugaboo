import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, autoComplete, Button, ChoiceGroup, TextField } from "@bugaboo/ui";

import { formatDay } from "../../../shared/format.ts";
import styles from "./AccessKeys.module.css";
import { choiceHelp, choiceName, plainChoices } from "./choice.ts";
import type { Draft } from "./draft.ts";

const DIGITS_OF_YEAR = 4;
const DIGITS_OF_PART = 2;

/** The day `days` after today (as milliseconds since 1970), by the calendar of the person's own clock, not by UTC. */
function localDay(days: number): number {
  const today = new Date();
  return new Date(today.getFullYear(), today.getMonth(), today.getDate() + days).getTime();
}

/** A day as the date field writes it (`2027-03-01`). */
function dayText(moment: number): string {
  const day = new Date(moment);
  const year = String(day.getFullYear()).padStart(DIGITS_OF_YEAR, "0");
  const month = String(day.getMonth() + 1).padStart(DIGITS_OF_PART, "0");
  const date = String(day.getDate()).padStart(DIGITS_OF_PART, "0");
  return `${year}-${month}-${date}`;
}

export interface PartProps {
  readonly draft: Draft;
}

/** The name, and what the key may do. */
export function Basics({ draft }: PartProps) {
  return (
    <>
      <TextField
        label={t("access-keys.name")}
        description={t("access-keys.name-help")}
        requiredMessage={t("validation.required")}
        value={draft.name}
        onChange={draft.setName}
      />
      {draft.single ? null : (
        <ChoiceGroup
          legend={t("access-keys.may")}
          choices={plainChoices.map((value) => ({ value, label: t(choiceName(value)) }))}
          value={draft.choice}
          onChange={draft.setChoice}
        />
      )}
      <p className={styles.muted}>{t(choiceHelp(draft.single ? "permissions" : draft.choice))}</p>
      <p className={styles.muted}>{t("access-keys.never")}</p>
    </>
  );
}

export interface ExpiryProps extends PartProps {
  readonly rules: Schema<"AccessKeyRulesBody">;
}

/** The day the key runs out: the default, or a day of the person's own, up to the longest time. */
export function Expiry({ draft, rules }: ExpiryProps) {
  // The latest day is at least tomorrow, and the day filled in is never later than it, even when
  // the organization's default is as long as its longest time.
  const latestAt = Math.max(rules.longest_days - 1, 1);
  const presetAt = Math.min(Math.max(rules.default_days, 1), latestAt);
  const latest = dayText(localDay(latestAt));
  return (
    <TextField
      label={t("access-keys.expires")}
      description={t("access-keys.expires-help", {
        days: rules.default_days,
        latest: formatDay(localDay(latestAt)),
      })}
      requiredMessage={t("validation.required")}
      date={{ min: dayText(localDay(1)), max: latest }}
      value={draft.day ?? dayText(localDay(presetAt))}
      onChange={draft.setDay}
    />
  );
}

export interface SendProps extends PartProps {
  /** What went wrong with the single permissions (none ticked), if anything. */
  readonly needOne: boolean;
  /** The text of the problem the server named, if it refused. */
  readonly problem: string | null;
  readonly isPending: boolean;
  readonly onCancel: () => void;
}

/** The password again, a problem if there is one, and the two buttons. */
export function Send({ draft, needOne, problem, isPending, onCancel }: SendProps) {
  return (
    <>
      <TextField
        label={t("field.confirm-password")}
        description={t("field.confirm-password-help")}
        password
        autoComplete={autoComplete.currentPassword}
        requiredMessage={t("validation.required")}
        value={draft.password}
        onChange={draft.setPassword}
      />
      {needOne ? <Alert danger>{t("access-keys.pick-one")}</Alert> : null}
      {problem === null ? null : <Alert danger>{problem}</Alert>}
      <div className={styles.row}>
        <Button primary submit isPending={isPending}>
          {t("access-keys.submit")}
        </Button>
        <Button onPress={onCancel}>{t("access-keys.cancel")}</Button>
      </div>
    </>
  );
}
