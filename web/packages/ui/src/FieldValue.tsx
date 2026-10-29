import type { choiceColorWords, lookWords } from "@bugaboo/api-client";
import type { ReactNode } from "react";

import { IssueTypeIcon, type IssueTypeIconName } from "./IssueTypeIcon.tsx";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";
import { wordsOnly } from "./words.ts";

/** The color of a choice: one of the category tokens of the theme (`cat1` … `cat12`). */
export type ChoiceColorName = (typeof choiceColorWords)[number];

/** How a value of an own field is shown (CF-04): a word of the server's list. */
export type FieldLook = (typeof lookWords)[number];

export interface FieldValueProps {
  readonly look: FieldLook;
  /** What the value says, already in the person's language. */
  readonly text: string;
  /** The color of the choice, for a badge and a pill; none shows the neutral color. */
  readonly color?: ChoiceColorName | undefined;
  /** The icon of the choice, for a badge and a pill. */
  readonly icon?: IssueTypeIconName | undefined;
  /** For a progress bar: how far it is, from 0 to 1. */
  readonly fraction?: number | undefined;
  /** For an avatar: the name its letters come from, when the text is more than the name (*Me (Ada Admin)*). */
  readonly initialsFrom?: string | undefined;
}

/** The first letters of the first two words of a name: what an avatar without a picture shows. */
function initialsOf(name: string): string {
  return wordsOnly(name)
    .slice(0, 2)
    .map((word) => Array.from(word)[0] ?? "")
    .join("")
    .toUpperCase();
}

/** The colored label of a badge and of a pill: the choice's icon, if it has one, and its name. */
function Label({
  partName,
  color,
  icon,
  text,
}: Pick<FieldValueProps, "color" | "icon" | "text"> & {
  readonly partName: typeof part.fieldBadge | typeof part.fieldPill;
}) {
  return (
    <span {...partOf(partName, { variants: color === undefined ? [] : [color] })}>
      {icon === undefined ? null : <IssueTypeIcon name={icon} />}
      {text}
    </span>
  );
}

function badge(props: FieldValueProps): ReactNode {
  return <Label {...props} partName={part.fieldBadge} />;
}

function pill(props: FieldValueProps): ReactNode {
  return <Label {...props} partName={part.fieldPill} />;
}

function progressBar({ text, fraction }: FieldValueProps): ReactNode {
  return (
    <span {...partOf(part.fieldProgress)}>
      <progress
        {...partOf(part.fieldProgressBar)}
        value={Math.min(1, Math.max(0, fraction ?? 0))}
        max={1}
        aria-label={text}
      />
      {text}
    </span>
  );
}

function avatar({ text, initialsFrom }: FieldValueProps): ReactNode {
  return (
    <span {...partOf(part.fieldAvatar)}>
      <span {...partOf(part.fieldAvatarFace)} aria-hidden>
        {initialsOf(initialsFrom ?? text)}
      </span>
      {text}
    </span>
  );
}

function plainText({ text }: FieldValueProps): ReactNode {
  return <span {...partOf(part.fieldText)}>{text}</span>;
}

/** How each look shows a value: one function for each look, so a new look cannot be forgotten. */
const looks: Readonly<Record<FieldLook, (props: FieldValueProps) => ReactNode>> = {
  badge,
  pill,
  "plain-text": plainText,
  "progress-bar": progressBar,
  avatar,
};

/**
 * The value of an own field in the look the field has (CF-04): a badge, a pill, plain text, a
 * progress bar or an avatar. Colors are the category tokens of the theme, so every look reads
 * in all four themes (STY-CONTRAST).
 */
export function FieldValue(props: FieldValueProps) {
  return looks[props.look](props);
}
