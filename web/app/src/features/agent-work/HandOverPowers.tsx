import { t } from "@bugaboo/messages";
import { Checkbox, partOf, part } from "@bugaboo/ui";

import { fieldNameOf } from "../../shared/field-names.ts";
import { useFields, useFieldTypes } from "../fields/index.ts";
import { nameableFields, type DraftProps } from "./hand-over-draft.ts";

/** The fields an agent can be given to change, one box each. */
function FieldBoxes({ draft, onChange }: DraftProps) {
  const own = useFields().data?.fields ?? [];
  const kinds = useFieldTypes().data?.kinds ?? [];
  return (
    <div {...partOf(part.agentWorkGroup, { variants: ["nested"] })}>
      {nameableFields(own, kinds).map(({ code, name }) => (
        <Checkbox
          key={code}
          label={fieldNameOf({ code, name })}
          isSelected={draft.fields.includes(code)}
          onChange={(on) => {
            onChange({
              fields: on ? [...draft.fields, code] : draft.fields.filter((each) => each !== code),
            });
          }}
        />
      ))}
    </div>
  );
}

/**
 * What the agent may do (UX §4.17): what it always may is said in words, the rest are boxes. Changing
 * fields asks which fields; the status and sub-issues are boxes of their own.
 */
export function HandOverPowers({ draft, onChange }: DraftProps) {
  return (
    <fieldset {...partOf(part.agentWorkGroup)}>
      <legend {...partOf(part.agentWorkGroupName)}>{t("agent-work.may-do")}</legend>
      <p {...partOf(part.muted)}>{t("agent-work.always")}</p>
      <Checkbox
        label={t("agent-work.power-change-status")}
        isSelected={draft.changeStatus}
        onChange={(on) => {
          onChange({ changeStatus: on });
        }}
      />
      <Checkbox
        label={t("agent-work.power-change-fields")}
        isSelected={draft.changeFields}
        onChange={(on) => {
          onChange({ changeFields: on });
        }}
      />
      {draft.changeFields ? <FieldBoxes draft={draft} onChange={onChange} /> : null}
      <Checkbox
        label={t("agent-work.power-create-sub-issues")}
        isSelected={draft.createSubIssues}
        onChange={(on) => {
          onChange({ createSubIssues: on });
        }}
      />
    </fieldset>
  );
}
