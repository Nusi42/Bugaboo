import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  ChoiceGroup,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import type { Schema } from "@bugaboo/api-client";
import { useMutation } from "@tanstack/react-query";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";
import {
  createRelation,
  deleteRelation,
  relationChoices,
  type BlockProps,
  type RelationChoice,
} from "./relations.ts";
import { exactName } from "../../shared/same-name.ts";
import { useNameKey } from "../../shared/use-name-key.ts";

/** What *Add relation* shows after the third step: the relation that was made, with *Undo*. */
function Added({
  readableId,
  other,
  name,
  relation,
  onUndo,
}: {
  readonly readableId: string;
  readonly other: string;
  readonly name: string;
  readonly relation: string;
  readonly onUndo: (relation: string) => void;
}) {
  return (
    <Alert tone={alertTone.success}>
      <span>{t("related-issues.added", { key: readableId, name, other })}</span>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          onUndo(relation);
        }}
      >
        {t("common.undo")}
      </Button>
    </Alert>
  );
}

/** The two steps before the relation is made: pick the words, then name the other issue. */
function Steps({
  choices,
  chosen,
  onChoose,
  other,
  onOther,
  onBack,
  onCancel,
  pending,
}: {
  readonly choices: readonly RelationChoice[];
  readonly chosen: string | null;
  readonly onChoose: (key: string) => void;
  readonly other: string;
  readonly onOther: (other: string) => void;
  readonly onBack: () => void;
  readonly onCancel: () => void;
  readonly pending: boolean;
}) {
  return (
    <>
      <ChoiceGroup
        legend={t("related-issues.kind-field")}
        choices={choices.map((choice) => ({ value: choice.key, label: choice.name }))}
        value={chosen ?? ""}
        onChange={onChoose}
      />
      {chosen === null ? null : (
        <TextField
          label={t("common.issue-id")}
          requiredMessage={t("error.field.required")}
          value={other}
          onChange={onOther}
        />
      )}
      <div {...partOf(part.row)}>
        {chosen === null ? null : (
          <SubmitButton isPending={pending}>{t("related-issues.add-it")}</SubmitButton>
        )}
        <Button look={buttonLook.quiet} onPress={chosen === null ? onCancel : onBack}>
          {chosen === null ? t("common.cancel") : t("related-issues.back")}
        </Button>
      </div>
    </>
  );
}

/** What the three steps keep: the words picked, the other issue, and the relation just made. */
function useAddRelation(readableId: string) {
  const kinds = useRelationKinds();
  const keyOf = useNameKey() ?? exactName;
  const choices = relationChoices(relationKindLinesOf(kinds.data?.pages), keyOf);
  const [chosen, setChosen] = useState<string | null>(null);
  const [other, setOther] = useState("");
  const [made, setMade] = useState<{ relation: string; other: string; name: string } | null>(null);
  const add = useMutation({
    mutationFn: (body: Schema<"NewRelationBody">) => createRelation(readableId, body),
    onSuccess: (relation) => {
      setMade({ relation: relation.id, other: relation.issue.readable_id, name: relation.name });
      setOther("");
      setChosen(null);
    },
  });
  const remove = useMutation({
    mutationFn: deleteRelation,
    onSuccess: () => {
      setMade(null);
    },
  });
  const choice = choices.find((each) => each.key === chosen);
  const submit = () => {
    if (choice !== undefined) {
      add.mutate({ other: other.trim(), relation_kind: choice.kind, direction: choice.direction });
    }
  };
  return { kinds, choices, chosen, setChosen, other, setOther, made, add, remove, submit };
}

/**
 * *Add relation* in three small steps (UX §4.4): pick the words, find the issue, and see it done
 * with *Undo*. The relation then shows on both issues, each in its own words.
 */
export function AddRelation({
  readableId,
  onClose,
}: BlockProps & { readonly onClose: () => void }) {
  const state = useAddRelation(readableId);
  if (state.kinds.isSuccess && state.choices.length === 0) {
    return <p {...partOf(part.muted)}>{t("related-issues.nothing-to-pick")}</p>;
  }
  return (
    <Form onSubmit={state.submit}>
      <Steps
        choices={state.choices}
        chosen={state.chosen}
        onChoose={state.setChosen}
        other={state.other}
        onOther={state.setOther}
        onBack={() => {
          state.setChosen(null);
        }}
        onCancel={onClose}
        pending={state.add.isPending}
      />
      {state.add.isError ? (
        <Alert tone={alertTone.danger}>{messageOf(state.add.error)}</Alert>
      ) : null}
      {state.made === null ? null : (
        <Added
          readableId={readableId}
          other={state.made.other}
          name={state.made.name}
          relation={state.made.relation}
          onUndo={(relation) => {
            state.remove.mutate(relation);
          }}
        />
      )}
    </Form>
  );
}
