import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Card,
  FieldValue,
  Form,
  SelectField,
  SubmitButton,
  pageAttributes,
  partOf,
  part,
  themeIds,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { QueryGate } from "../shell/index.ts";
import { lookName, useFieldTypes, type Field, type LookWord } from "./fields.ts";
import { fieldChange, useChangeField } from "./use-change-field.ts";

/** How far the sample of a progress bar is. */
const SAMPLE_FRACTION = 0.6;
/** How many choices a sample shows at most. */
const SAMPLE_CHOICES = 4;

/** One value in the look, for the preview: the choices of the field, or a sample of the kind. */
function Samples({ field, look }: { readonly field: Field; readonly look: LookWord }) {
  const choices = field.choices.filter((choice) => !choice.retired).slice(0, SAMPLE_CHOICES);
  if (choices.length > 0) {
    return (
      <>
        {choices.map((choice) => (
          <FieldValue
            key={choice.id}
            look={look}
            text={choice.name}
            color={choice.color}
            icon={choice.icon ?? undefined}
          />
        ))}
      </>
    );
  }
  return <FieldValue look={look} text={t("fields.look-sample")} fraction={SAMPLE_FRACTION} />;
}

/** The look to choose, with the preview in all four themes next to it. */
function LookForm({
  field,
  looks,
}: {
  readonly field: Field;
  readonly looks: readonly LookWord[];
}) {
  const [look, setLook] = useState<LookWord>(field.look);
  const change = useChangeField();
  return (
    <Form
      onSubmit={() => {
        change.mutate(fieldChange(field, { look }));
      }}
    >
      <SelectField
        label={t("fields.look-label")}
        choices={looks.map((each) => ({ value: each, label: lookName(each) }))}
        value={look}
        onChange={setLook}
      />
      <fieldset {...partOf(part.fieldSample)}>
        <legend {...partOf(part.choiceGroupLegend)}>{t("fields.look-preview")}</legend>
        {themeIds.map((theme) => (
          <div
            key={theme}
            {...partOf(part.fieldSampleTheme)}
            {...{ [pageAttributes.previewLook]: theme }}
          >
            <span>{t(`theme.${theme}`)}</span>
            <Samples field={field} look={look} />
          </div>
        ))}
      </fieldset>
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={change.isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}

/**
 * *Look* of the field page (CF-04): how its values are shown, with a preview in all four themes
 * side by side, so a color that cannot be read in one of them is seen before it is chosen.
 */
export function LookSection({ field }: { readonly field: Field }) {
  const types = useFieldTypes();
  return (
    <Card heading={t("fields.look")}>
      <QueryGate error={types.error} data={types.data}>
        {(known) => (
          <LookForm
            field={field}
            looks={known.kinds.find((each) => each.kind === field.kind)?.looks ?? [field.look]}
          />
        )}
      </QueryGate>
    </Card>
  );
}
