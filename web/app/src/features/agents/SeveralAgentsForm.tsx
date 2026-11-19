import { agentsAtOnceMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Button,
  DelayedSkeleton,
  fieldKind,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { keyRulesOptions, KeyExpiry, useKeyDraft } from "../my-settings/index.ts";
import { namesOf, type CreatedAgent } from "./several-agents.ts";
import { useSeveralRequest } from "./use-several-request.ts";
import { PasswordAgain } from "./PasswordAgain.tsx";

export interface SeveralAgentsFormProps {
  /** The agents were created: here they are with their keys, which are not shown again. */
  readonly onCreated: (created: readonly CreatedAgent[]) => void;
  readonly onCancel: () => void;
}

/**
 * *Create several agents*: one name a line, when their keys run out, and the password once. Every
 * agent gets one key; when one cannot be created, none is.
 */
export function SeveralAgentsForm({ onCreated, onCancel }: SeveralAgentsFormProps) {
  const rules = useQuery(keyRulesOptions());
  const draft = useKeyDraft();
  const [text, setText] = useState("");
  const request = useSeveralRequest(onCreated);
  if (rules.data === undefined) {
    return <DelayedSkeleton />;
  }
  const loaded = rules.data;
  const names = namesOf(text);
  const tooMany = names.length > agentsAtOnceMost;
  return (
    <Form
      onSubmit={() => {
        request.send(names, draft, loaded);
      }}
    >
      <TextField
        label={t("agents.several-names")}
        description={t("agents.several-names-help", { most: agentsAtOnceMost })}
        kind={fieldKind.multiline}
        requiredMessage={t("error.field.required")}
        errorMessage={
          tooMany ? t("agents.several-too-many", { most: agentsAtOnceMost }) : undefined
        }
        value={text}
        onChange={setText}
      />
      <KeyExpiry draft={draft} rules={loaded} wrongDay={request.wrongDay} />
      <PasswordAgain
        help={t("agents.several-password-help")}
        value={draft.password}
        onChange={draft.setPassword}
        failure={request.failure}
      />
      <div {...partOf(part.row, { variants: ["stretch"] })}>
        <SubmitButton isPending={request.pending}>
          {t("agents.several-confirm", { count: names.length })}
        </SubmitButton>
        <Button onPress={onCancel}>{t("common.cancel")}</Button>
      </div>
    </Form>
  );
}
