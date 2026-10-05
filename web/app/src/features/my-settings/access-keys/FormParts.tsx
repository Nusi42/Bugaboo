import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, autoComplete, Button, ChoiceGroup, TextField } from "@bugaboo/ui";

import styles from "./AccessKeys.module.css";
import { choiceHelp, choiceName, plainChoices } from "./choice.ts";
import type { Draft } from "./draft.ts";

const MS_PER_DAY = 86_400_000;
const DAY_TEXT_LENGTH = 10;

/** A day as the date field writes it (`2027-03-01`), `days` after today. */
function dayAfter(days: number): string {
  return new Date(Date.now() + days * MS_PER_DAY).toISOString().slice(0, DAY_TEXT_LENGTH);
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
    </>
  );
}

export interface ExpiryProps extends PartProps {
  readonly rules: Schema<"AccessKeyRulesBody">;
}

/** The day the key runs out: the default, or a day of the person's own, up to the longest time. */
export function Expiry({ draft, rules }: ExpiryProps) {
  const latest = dayAfter(rules.longest_days - 1);
  return (
    <TextField
      label={t("access-keys.expires")}
      description={t("access-keys.expires-help", { days: rules.default_days, latest })}
      requiredMessage={t("validation.required")}
      date={{ min: dayAfter(1), max: latest }}
      value={draft.day ?? dayAfter(rules.default_days)}
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
