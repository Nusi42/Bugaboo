import { fieldForWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  ChoiceGroup,
  Form,
  SubmitButton,
  Switch,
  TextField,
  fieldKind,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { QueryGate } from "../shell/index.ts";
import { typeExample, typeName, useFieldTypes, type KindWord, type TypeInfo } from "./fields.ts";
import { useCreateField } from "./use-change-field.ts";
import { wordsOfLines } from "./value-text.ts";

const forChoices = fieldForWords.map((word) => ({ value: word, label: t(`field-for.${word}`) }));

type ForWord = (typeof fieldForWords)[number];

/** The two questions a field starts with: what kind of information, and what it is for. */
function Questions({
  kinds,
  kind,
  setKind,
  appliesTo,
  setAppliesTo,
}: {
  readonly kinds: readonly TypeInfo[];
  readonly kind: KindWord;
  readonly setKind: (kind: KindWord) => void;
  readonly appliesTo: ForWord;
  readonly setAppliesTo: (appliesTo: ForWord) => void;
}) {
  return (
    <>
      <ChoiceGroup
        legend={t("fields.kind-question")}
        choices={kinds.map((each) => ({ value: each.kind, label: typeName(each.kind) }))}
        value={kind}
        onChange={setKind}
      />
      <ChoiceGroup
        legend={t("fields.for-question")}
        choices={forChoices}
        value={appliesTo}
        onChange={setAppliesTo}
      />
    </>
  );
}

/** What is typed so far. */
interface Draft {
  readonly kind: KindWord;
  readonly appliesTo: ForWord;
  readonly name: string;
  readonly help: string;
  readonly required: boolean;
  readonly choices: string;
}

const emptyDraft: Draft = {
  kind: "text",
  appliesTo: "issues",
  name: "",
  help: "",
  required: false,
  choices: "",
};

/** The request that makes the field: the choices only count for a kind that has them. */
function bodyOf(draft: Draft, withChoices: boolean) {
  return {
    name: draft.name.trim(),
    help: draft.help.trim(),
    kind: draft.kind,
    applies_to: draft.appliesTo,
    rules: { required: draft.required },
    choices: withChoices ? wordsOfLines(draft.choices).map((each) => ({ name: each })) : [],
  };
}

/** The choices of a *Pick one* or *Pick several* field, one on each line. */
function Choices({
  value,
  onChange,
}: {
  readonly value: string;
  readonly onChange: (value: string) => void;
}) {
  return (
    <TextField
      label={t("fields.field-choices")}
      description={t("fields.field-choices-help")}
      kind={fieldKind.multiline}
      value={value}
      onChange={onChange}
    />
  );
}

/** The form of a new field, once the kinds are known. */
function CreateFieldFields({ kinds }: { readonly kinds: readonly TypeInfo[] }) {
  const [draft, setDraft] = useState(emptyDraft);
  const create = useCreateField();
  const set =
    <Key extends keyof Draft>(key: Key) =>
    (value: Draft[Key]) => {
      setDraft({ ...draft, [key]: value });
    };
  const hasChoices = kinds.find((each) => each.kind === draft.kind)?.traits.includes("has-choices");
  return (
    <Form
      onSubmit={() => {
        create.mutate({ body: bodyOf(draft, hasChoices === true) });
      }}
    >
      <Questions
        kinds={kinds}
        kind={draft.kind}
        setKind={set("kind")}
        appliesTo={draft.appliesTo}
        setAppliesTo={set("appliesTo")}
      />
      <TextField
        label={t("fields.field-name")}
        description={typeExample(draft.kind)}
        hintAbove
        requiredMessage={t("error.field.required")}
        value={draft.name}
        onChange={set("name")}
      />
      <TextField label={t("fields.field-help")} value={draft.help} onChange={set("help")} />
      {hasChoices === true ? <Choices value={draft.choices} onChange={set("choices")} /> : null}
      <Switch
        label={t("fields.field-required")}
        isSelected={draft.required}
        onChange={set("required")}
      />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("fields.create")}</SubmitButton>
      </div>
    </Form>
  );
}

/**
 * *Create a field* (UX §5.3): it starts with "What kind of information?", the name comes next, and
 * the choices of a *Pick one* or *Pick several* field are typed one on each line. The field page
 * has the rest: rules, look, where it is used.
 */
export function CreateFieldForm() {
  const types = useFieldTypes();
  return (
    <QueryGate error={types.error} data={types.data}>
      {(known) => <CreateFieldFields kinds={known.kinds} />}
    </QueryGate>
  );
}
