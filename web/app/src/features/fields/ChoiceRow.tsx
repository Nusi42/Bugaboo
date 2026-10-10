import { choiceColorWords, issueIconWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  buttonLook,
  ColorChoice,
  ConfirmDialog,
  FieldValue,
  Form,
  IconButton,
  iconName,
  SelectField,
  SubmitButton,
  Switch,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { plainText, type Choice, type Field } from "./fields.ts";
import { useChangeChoice } from "./use-change-field.ts";

/** Words of the HTML standard and marks that are not text for people. */
const noIcon = "none";
const badge = "badge";

const iconChoices = [
  { value: noIcon, label: t("fields.no-icon") },
  ...issueIconWords.map((word) => ({ value: word, label: t(`issue-icon.${word}`) })),
];

const colorChoices = choiceColorWords.map((word) => ({
  value: word,
  label: t(`choice-color.${word}`),
}));

/** Sends a change of the choice, on the version of the field the person looked at. */
type Send = (more: Readonly<Record<string, unknown>>) => void;

export interface ChoiceRowProps {
  readonly field: Field;
  readonly choice: Choice;
  readonly first: boolean;
  readonly last: boolean;
  readonly onMove: (by: -1 | 1) => void;
  /** The other choices a value can be combined into. */
  readonly others: readonly Choice[];
  readonly onCombine: (into: string) => void;
}

/** The choice as its field shows it, with the buttons that move it. */
function ChoiceTitle({
  field,
  choice,
  first,
  last,
  onMove,
}: Pick<ChoiceRowProps, "field" | "choice" | "first" | "last" | "onMove">) {
  return (
    <>
      <FieldValue
        look={field.look === plainText ? badge : field.look}
        text={choice.name}
        color={choice.color}
        icon={choice.icon ?? undefined}
      />
      <IconButton
        icon={iconName.back}
        label={t("fields.choice-up", { name: choice.name })}
        onPress={() => {
          if (!first) {
            onMove(-1);
          }
        }}
      />
      <IconButton
        icon={iconName.next}
        label={t("fields.choice-down", { name: choice.name })}
        onPress={() => {
          if (!last) {
            onMove(1);
          }
        }}
      />
    </>
  );
}

/** The name of the choice, saved with its own button. */
function ChoiceName({
  choice,
  send,
  pending,
}: {
  readonly choice: Choice;
  readonly send: Send;
  readonly pending: boolean;
}) {
  const [name, setName] = useState(choice.name);
  return (
    <Form
      onSubmit={() => {
        send({ name: name.trim() });
      }}
    >
      <TextField
        label={t("fields.choice-name")}
        requiredMessage={t("validation.required")}
        value={name}
        onChange={setName}
      />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={pending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}

/** The color and icon of the choice, and the switch that retires it. */
function ChoiceLook({ choice, send }: { readonly choice: Choice; readonly send: Send }) {
  return (
    <>
      <ColorChoice
        legend={t("fields.choice-color", { name: choice.name })}
        colors={colorChoices}
        value={choice.color}
        sample={t("fields.color-sample")}
        onChange={(color) => {
          send({ color });
        }}
      />
      <SelectField
        label={t("fields.choice-icon", { name: choice.name })}
        choices={iconChoices}
        value={choice.icon ?? noIcon}
        onChange={(icon) => {
          send({ icon: icon === noIcon ? null : icon });
        }}
      />
      <Switch
        label={choice.retired ? t("fields.choice-offer-again") : t("fields.choice-retire")}
        isSelected={choice.retired}
        onChange={(retired) => {
          send({ retired });
        }}
      />
    </>
  );
}

/** *Combine with another choice*: every value of this choice moves to the one picked, after a question. */
function ChoiceCombine({
  choice,
  others,
  onCombine,
}: Pick<ChoiceRowProps, "choice" | "others" | "onCombine">) {
  const [into, setInto] = useState(others[0]?.id ?? "");
  return (
    <div {...partOf(part.row)}>
      <SelectField
        label={t("fields.combine-into")}
        choices={others.map((other) => ({ value: other.id, label: other.name }))}
        value={into}
        onChange={setInto}
      />
      <ConfirmDialog
        triggerLabel={t("fields.combine")}
        triggerName={t("fields.combine-name", { name: choice.name })}
        title={t("fields.combine-title", { name: choice.name })}
        text={t("fields.combine-text", {
          name: choice.name,
          into: others.find((other) => other.id === into)?.name ?? "",
        })}
        confirmLabel={t("fields.combine")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          onCombine(into);
        }}
      />
    </div>
  );
}

/** One choice of the field: its name, color, icon, place in the order, *Combine* and *Retire*. */
export function ChoiceRow(props: ChoiceRowProps) {
  const { field, choice, others } = props;
  const change = useChangeChoice();
  const send: Send = (more) => {
    change.mutate({
      params: { path: { field: field.id, choice: choice.id } },
      body: { version: field.version, ...more },
    });
  };
  return (
    <li {...partOf(part.fieldChoice)}>
      <ChoiceTitle {...props} />
      <ChoiceName choice={choice} send={send} pending={change.isPending} />
      <ChoiceLook choice={choice} send={send} />
      {others.length > 0 && !choice.retired ? (
        <ChoiceCombine choice={choice} others={others} onCombine={props.onCombine} />
      ) : null}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </li>
  );
}
