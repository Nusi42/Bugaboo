import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Card,
  Form,
  SubmitButton,
  Switch,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useFieldTypes, type OwnField } from "./fields.ts";
import { fieldChange, useChangeField } from "./use-change-field.ts";
import { ValueEditor, type Sent } from "./ValueEditor.tsx";

/** The switches of the basics: required, and never sent to the assistant. */
function Flags({ field }: { readonly field: OwnField }) {
  const change = useChangeField();
  return (
    <>
      <Switch
        label={t("fields.field-required")}
        isSelected={field.rules.required === true}
        onChange={(required) => {
          change.mutate(fieldChange(field, { rules: { ...field.rules, required } }));
        }}
      />
      <Switch
        label={t("fields.never-to-assistant")}
        isSelected={field.never_to_assistant}
        onChange={(never) => {
          change.mutate(fieldChange(field, { never_to_assistant: never }));
        }}
      />
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </>
  );
}

/** The value a new issue or project starts with. */
function DefaultValue({ field }: { readonly field: OwnField }) {
  const change = useChangeField();
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("fields.default-help")}</p>
      <ValueEditor
        key={field.version}
        field={field}
        value={field.default_value}
        isPending={change.isPending}
        onCommit={(value: Sent) => {
          change.mutate(fieldChange(field, { default_value: value }));
        }}
      />
    </div>
  );
}

/** *Basics* of the field page: name, help text, required, the default and the mark for the assistant. */
export function BasicsSection({ field }: { readonly field: OwnField }) {
  const [name, setName] = useState(field.name);
  const [help, setHelp] = useState(field.help);
  const change = useChangeField();
  const types = useFieldTypes();
  const takesDefault = types.data?.kinds
    .find((each) => each.kind === field.kind)
    ?.traits.includes("takes-default");
  return (
    <Card heading={t("fields.basics")}>
      <Form
        onSubmit={() => {
          change.mutate(fieldChange(field, { name: name.trim(), help: help.trim() }));
        }}
      >
        <TextField
          label={t("fields.field-name")}
          requiredMessage={t("validation.required")}
          value={name}
          onChange={setName}
        />
        <TextField label={t("fields.field-help")} value={help} onChange={setHelp} />
        <div {...partOf(part.row)}>
          <SubmitButton isPending={change.isPending}>{t("common.save")}</SubmitButton>
        </div>
      </Form>
      <Flags field={field} />
      {takesDefault === true ? <DefaultValue field={field} /> : null}
    </Card>
  );
}
