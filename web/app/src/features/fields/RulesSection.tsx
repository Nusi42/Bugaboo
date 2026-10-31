import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { WithLengthReader, type LengthReader } from "../../shared/with-length-reader.tsx";
import { QueryGate } from "../shell/index.ts";
import { useFieldTypes, type OwnField } from "./fields.ts";
import { boxOf, countOf, storedOf } from "./rule-values.ts";
import { fieldChange, useChangeField } from "./use-change-field.ts";
import { bound } from "./value-text.ts";

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
        const shape = { ranged, counted, pattern, dated: traits.includes("dated") };
        if (!ranged && !counted) {
          return null;
        }
        // The bounds of a length of time are read by the one reader, whose module loads here.
        return field.kind === "duration" ? (
          <WithLengthReader>
            {(readLength) => <RulesForm field={field} shape={shape} readLength={readLength} />}
          </WithLengthReader>
        ) : (
          <RulesForm field={field} shape={shape} />
        );
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
function rulesOf(
  field: OwnField,
  boxes: Boxes,
  shape: Shape,
  readLength: LengthReader | undefined,
): Record<string, unknown> | undefined {
  const lowest = shape.ranged ? storedOf(field, boxes.min, bound.lowest, readLength) : null;
  const highest = shape.ranged ? storedOf(field, boxes.max, bound.highest, readLength) : null;
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

interface RulesFormProps {
  readonly field: OwnField;
  readonly shape: Shape;
  /** The one reader of typed lengths, for the bounds of a length of time. */
  readonly readLength?: LengthReader;
}

function RulesForm({ field, shape, readLength }: RulesFormProps) {
  const [boxes, setBoxes] = useState(boxesOf(field));
  const [problem, setProblem] = useState<string | undefined>();
  const change = useChangeField();
  const submit = () => {
    const rules = rulesOf(field, boxes, shape, readLength);
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
