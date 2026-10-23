import { t } from "@bugaboo/messages";
import { FieldValue, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { ownValueText } from "../../shared/own-value-text.ts";
import { personName, type Person } from "../../shared/person.ts";
import { choiceOf, plainText, type Choice, type OwnField, type KindWord } from "./fields.ts";
import { memberOf } from "../../shared/reference.ts";

/** How far a number is between the lowest and the highest of the field, from 0 to 1 (a percent without a range). */
function fractionOf(field: OwnField, value: number): number {
  const { min, max } = field.rules;
  if (min !== undefined && min !== null && max !== undefined && max !== null && max > min) {
    return (value - min) / (max - min);
  }
  return value / 100;
}

/** Whether a value from the API is a list of texts. */
function textsOf(value: unknown): readonly string[] {
  return Array.isArray(value)
    ? value.filter((each): each is string => typeof each === "string")
    : [];
}

/** Whether an object from the API is a person as the server shows one (`PersonRef`). */
function isPerson(value: unknown): value is Person {
  return typeof memberOf(value, "shown") === "string";
}

/** One thing a person, group or issue value names, as the server resolved it for this viewer. */
function referenceText(value: unknown): string {
  if (typeof value === "object" && value !== null) {
    if (isPerson(value)) {
      return personName(value);
    }
    const name = memberOf(value, "name");
    const key = memberOf(value, "readable_id");
    if (typeof name === "string") {
      return name;
    }
    if (typeof key === "string") {
      return key;
    }
  }
  return t("fields.hidden-reference");
}

/** A choice in the look of its field. */
function ChoiceValue({
  field,
  choice,
}: {
  readonly field: OwnField;
  readonly choice: Choice | undefined;
}) {
  return (
    <FieldValue
      look={field.look}
      text={choice?.name ?? t("fields.unknown-choice")}
      color={choice?.color}
      icon={choice?.icon ?? undefined}
    />
  );
}

/** What a viewer of one kind of value is given. */
interface ViewProps {
  readonly field: OwnField;
  readonly value: unknown;
}

function plain(text: string): ReactNode {
  return <FieldValue look={plainText} text={text} />;
}

function viewText({ value }: ViewProps): ReactNode {
  return typeof value === "string" ? plain(value) : null;
}

function viewPickOne({ field, value }: ViewProps): ReactNode {
  return typeof value === "string" ? (
    <ChoiceValue field={field} choice={choiceOf(field, value)} />
  ) : null;
}

function viewPickSeveral({ field, value }: ViewProps): ReactNode {
  return (
    <span {...partOf(part.row)}>
      {textsOf(value).map((id) => (
        <ChoiceValue key={id} field={field} choice={choiceOf(field, id)} />
      ))}
    </span>
  );
}

function viewNumber({ field, value }: ViewProps): ReactNode {
  const text = ownValueText(field.kind, value);
  return text === undefined ? null : (
    <FieldValue look={field.look} text={text} fraction={fractionOf(field, Number(value))} />
  );
}

/** A value the screen words itself (yes or no, a length of time, a day, a moment) as plain text. */
function viewWorded({ field, value }: ViewProps): ReactNode {
  const text = ownValueText(field.kind, value);
  return text === undefined ? null : plain(text);
}

function viewReference({ field, value }: ViewProps): ReactNode {
  return <FieldValue look={field.look} text={referenceText(value)} />;
}

function viewReferences({ field, value }: ViewProps): ReactNode {
  return Array.isArray(value) ? (
    <span {...partOf(part.row)}>
      {value.map((each: unknown) => {
        const text = referenceText(each);
        return <FieldValue key={text} look={field.look} text={text} />;
      })}
    </span>
  ) : null;
}

/** How each kind of value is shown: one function for each kind, so a new kind cannot be forgotten. */
const views: Readonly<Record<KindWord, (props: ViewProps) => ReactNode>> = {
  text: viewText,
  "long-text": viewText,
  "web-address": viewText,
  "whole-number": viewNumber,
  decimal: viewNumber,
  "yes-no": viewWorded,
  date: viewWorded,
  "date-time": viewWorded,
  duration: viewWorded,
  "pick-one": viewPickOne,
  "pick-several": viewPickSeveral,
  person: viewReference,
  group: viewReference,
  issue: viewReference,
  people: viewReferences,
};

/**
 * The value of an own field in the look of the field (CF-04), as the server sent it for this
 * viewer: the JSON of its type. A field without a value shows nothing; the screen decides whether
 * to say so.
 */
export function ValueView({ field, value }: ViewProps): ReactNode {
  if (value === undefined || value === null) {
    return null;
  }
  return views[field.kind]({ field, value });
}
