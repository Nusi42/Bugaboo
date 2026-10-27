import { t } from "@bugaboo/messages";
import { wholeNumberIn } from "../../shared/number.ts";
import { Alert, alertTone, Card, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { durationText } from "../../shared/own-value-text.ts";
import { QueryGate } from "../shell/index.ts";
import { useFieldTypes, type OwnField } from "./fields.ts";
import { fieldChange, useChangeField } from "./use-change-field.ts";
import {
  boundDay,
  dayBound,
  decimalToMillionths,
  durationSeconds,
  millionthsToDecimal,
} from "./value-text.ts";

/** Whether the bounds of the field are days (a *Date* or a *Date and time*). */
function isDated(field: OwnField): boolean {
  return field.kind === "date" || field.kind === "date-time";
}

/** The lowest or highest value in a box, as people write it for this kind. */
function boxOf(field: OwnField, stored: number | null | undefined): string {
  if (stored === undefined || stored === null) {
    return "";
  }
  if (isDated(field)) {
    return boundDay(field.kind, stored);
  }
  if (field.kind === "decimal") {
    return millionthsToDecimal(stored);
  }
  return field.kind === "duration" ? durationText(stored) : String(stored);
}

/** What a box means in the unit the API stores; `null` for an empty box, `undefined` for text that is none. */
function storedOf(field: OwnField, text: string): number | null | undefined {
  const trimmed = text.trim();
  if (trimmed === "") {
    return null;
  }
  if (isDated(field)) {
    return dayBound(field.kind, trimmed);
  }
  if (field.kind === "decimal") {
    return decimalToMillionths(trimmed);
  }
  if (field.kind === "duration") {
    return durationSeconds(trimmed);
  }
  return wholeNumberIn(trimmed) ?? undefined;
}

/** A count of characters or values in a box: a whole number, or nothing. */
function countOf(text: string): number | null | undefined {
  const trimmed = text.trim();
  if (trimmed === "") {
    return null;
  }
  return /^\d+$/u.test(trimmed) ? Number(trimmed) : undefined;
}

interface Shape {
  /** The kind has a range of values. */
  readonly ranged: boolean;
  /** The kind has a length or a count of values. */
  readonly counted: boolean;
  /** The kind has a pattern. */
  readonly pattern: boolean;
  /** The bounds are days. */
  readonly dated: boolean;
}

/** *Rules* of the field page (CF-03): the allowed range, length and pattern, for the kinds that have them. */
export function RulesSection({ field }: { readonly field: OwnField }) {
  const types = useFieldTypes();
  return (
    <QueryGate error={types.error} data={types.data}>
      {(known) => {
        const traits = known.kinds.find((each) => each.kind === field.kind)?.traits ?? [];
        const ranged = traits.includes("range");
        const pattern = traits.includes("text");
        const counted = pattern || traits.includes("many");
        return ranged || counted ? (
          <RulesForm
            field={field}
            shape={{ ranged, counted, pattern, dated: traits.includes("dated") }}
          />
        ) : null;
      }}
    </QueryGate>
  );
}

/** What is typed in the boxes of the rules. */
interface Boxes {
  readonly min: string;
  readonly max: string;
  readonly shortest: string;
  readonly longest: string;
  readonly text: string;
}

function boxesOf(field: OwnField): Boxes {
  const { rules } = field;
  return {
    min: boxOf(field, rules.min),
    max: boxOf(field, rules.max),
    shortest:
      rules.min_length === undefined || rules.min_length === null ? "" : String(rules.min_length),
    longest:
      rules.max_length === undefined || rules.max_length === null ? "" : String(rules.max_length),
    text: rules.pattern ?? "",
  };
}

/** The rules the boxes say, or `undefined` when one box holds text that is no number. */
function rulesOf(field: OwnField, boxes: Boxes, shape: Shape): Record<string, unknown> | undefined {
  const lowest = shape.ranged ? storedOf(field, boxes.min) : null;
  const highest = shape.ranged ? storedOf(field, boxes.max) : null;
  const fewest = shape.counted ? countOf(boxes.shortest) : null;
  const most = shape.counted ? countOf(boxes.longest) : null;
  if (lowest === undefined || highest === undefined || fewest === undefined || most === undefined) {
    return undefined;
  }
  return {
    required: field.rules.required === true,
    ...(lowest === null ? {} : { min: lowest }),
    ...(highest === null ? {} : { max: highest }),
    ...(fewest === null ? {} : { min_length: fewest }),
    ...(most === null ? {} : { max_length: most }),
    ...(shape.pattern && boxes.text.trim() !== "" ? { pattern: boxes.text.trim() } : {}),
  };
}

interface BoxesProps {
  readonly boxes: Boxes;
  readonly onChange: (boxes: Boxes) => void;
}

/** The lowest and highest value. */
function RangeBoxes({ boxes, onChange, dated }: BoxesProps & { readonly dated: boolean }) {
  const day = dated ? { date: { min: "0001-01-01", max: "9999-12-31" } } : {};
  return (
    <>
      <TextField
        {...day}
        label={t("fields.rule-min")}
        value={boxes.min}
        onChange={(min) => {
          onChange({ ...boxes, min });
        }}
      />
      <TextField
        {...day}
        label={t("fields.rule-max")}
        value={boxes.max}
        onChange={(max) => {
          onChange({ ...boxes, max });
        }}
      />
    </>
  );
}

/** The fewest and most characters or values. */
function CountBoxes({ boxes, onChange }: BoxesProps) {
  return (
    <>
      <TextField
        label={t("fields.rule-min-length")}
        value={boxes.shortest}
        onChange={(shortest) => {
          onChange({ ...boxes, shortest });
        }}
      />
      <TextField
        label={t("fields.rule-max-length")}
        value={boxes.longest}
        onChange={(longest) => {
          onChange({ ...boxes, longest });
        }}
      />
    </>
  );
}

/** The boxes of the rules that a kind has. */
function RuleBoxes({ shape, ...rest }: BoxesProps & { readonly shape: Shape }) {
  return (
    <>
      {shape.ranged ? <RangeBoxes {...rest} dated={shape.dated} /> : null}
      {shape.counted ? <CountBoxes {...rest} /> : null}
      {shape.pattern ? (
        <TextField
          label={t("fields.rule-pattern")}
          description={t("fields.rule-pattern-help")}
          value={rest.boxes.text}
          onChange={(text) => {
            rest.onChange({ ...rest.boxes, text });
          }}
        />
      ) : null}
    </>
  );
}

function RulesForm({ field, shape }: { readonly field: OwnField; readonly shape: Shape }) {
  const [boxes, setBoxes] = useState(boxesOf(field));
  const [problem, setProblem] = useState<string | undefined>();
  const change = useChangeField();
  const submit = () => {
    const rules = rulesOf(field, boxes, shape);
    setProblem(rules === undefined ? t("fields.rules-not-a-number") : undefined);
    if (rules !== undefined) {
      change.mutate(fieldChange(field, { rules }));
    }
  };
  return (
    <Card heading={t("fields.rules")}>
      <Form onSubmit={submit}>
        <RuleBoxes shape={shape} boxes={boxes} onChange={setBoxes} />
        {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
        <div {...partOf(part.row)}>
          <SubmitButton isPending={change.isPending}>{t("common.save")}</SubmitButton>
        </div>
      </Form>
    </Card>
  );
}
