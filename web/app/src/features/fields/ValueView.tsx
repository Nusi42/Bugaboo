import { t } from "@bugaboo/messages";
import { FieldValue, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { formatDate, formatDateTime, formatNumber } from "../../shared/format.ts";
import { personName, type Person } from "../../shared/person.ts";
import { choiceOf, plainText, type Choice, type OwnField, type KindWord } from "./fields.ts";
import { durationText } from "./value-text.ts";

/** A day from the API (`2026-10-10`) as a moment, for the way the person writes days. */
function momentOfDay(day: string): string {
  return `${day}T00:00:00Z`;
}

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

/** A member of an object from the API, whatever it holds. */
function memberOf(value: unknown, name: string): unknown {
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  const found: unknown = Reflect.get(value, name);
  return found;
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
    const key = memberOf(value, "key");
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
  return typeof value === "number" || typeof value === "string" ? (
    <FieldValue
      look={field.look}
      text={typeof value === "number" ? formatNumber(value) : value}
      fraction={fractionOf(field, Number(value))}
    />
  ) : null;
}

function viewYesNo({ value }: ViewProps): ReactNode {
  return plain(value === true ? t("fields.yes") : t("fields.no"));
}

function viewDuration({ value }: ViewProps): ReactNode {
  return typeof value === "number" ? plain(durationText(value)) : null;
}

function viewDate({ value }: ViewProps): ReactNode {
  return typeof value === "string" ? plain(formatDate(momentOfDay(value))) : null;
}

function viewDateTime({ value }: ViewProps): ReactNode {
  return typeof value === "string" ? plain(formatDateTime(value)) : null;
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
  "yes-no": viewYesNo,
  date: viewDate,
  "date-time": viewDateTime,
  duration: viewDuration,
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
