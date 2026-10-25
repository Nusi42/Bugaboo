import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Button,
  ChoiceGroup,
  fieldKind,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";

import { formatCalendarDay } from "../../../shared/format.ts";
import { useNow } from "../../../shared/use-now.ts";
import { choiceHelp, choiceName, plainChoices } from "./choice.ts";
import type { Draft } from "./draft.ts";
import { defaultDaysOf, earliestDay, latestDay, presetDay } from "./expiry-day.ts";

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
        requiredMessage={t("error.field.required")}
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
      <p {...partOf(part.muted)}>{t(choiceHelp(draft.single ? "permissions" : draft.choice))}</p>
      <p {...partOf(part.muted)}>{t("access-keys.never")}</p>
    </>
  );
}

export interface ExpiryProps extends PartProps {
  readonly rules: Schema<"AccessKeyRulesBody">;
  /** The day that was filled in is no day: the field says so. */
  readonly wrongDay: boolean;
}

/** The day the key runs out: the default, or a day of the person's own, up to the longest time. */
export function Expiry({ draft, rules, wrongDay }: ExpiryProps) {
  // The days below are read from the clock: the form follows it, so one left open past midnight
  // shows the day the key would run out on now, which is the day the server counts from.
  useNow();
  return (
    <TextField
      label={t("access-keys.expires")}
      description={t("access-keys.expires-help", {
        days: defaultDaysOf(rules),
        latest: formatCalendarDay(latestDay(rules)),
      })}
      requiredMessage={t("error.field.required")}
      errorMessage={wrongDay ? t("error.field.wrong-value") : undefined}
      date={{ min: earliestDay(), max: latestDay(rules) }}
      value={draft.day ?? presetDay(rules)}
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
        kind={fieldKind.password}
        autoComplete={autoComplete.currentPassword}
        requiredMessage={t("error.field.required")}
        value={draft.password}
        onChange={draft.setPassword}
      />
      {needOne ? <Alert tone={alertTone.danger}>{t("access-keys.pick-one")}</Alert> : null}
      {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
      <div {...partOf(part.row, { variants: ["stretch"] })}>
        <SubmitButton isPending={isPending}>{t("access-keys.submit")}</SubmitButton>
        <Button onPress={onCancel}>{t("common.cancel")}</Button>
      </div>
    </>
  );
}
