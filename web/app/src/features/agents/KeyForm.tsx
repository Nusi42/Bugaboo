import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Button,
  fieldKind,
  Form,
  Skeleton,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../shared/errors.ts";
import { keyRulesOptions, KeyExpiry, useKeyDraft } from "../my-settings/index.ts";
import type { AgentKey } from "./agents.ts";
import { useKeyRequest } from "./use-key-request.ts";

export interface KeyFormProps {
  readonly agent: string;
  /** The key a new one takes the place of; none for a key that is new. */
  readonly renewing: AgentKey | null;
  /** The key was made: here is the whole key, which is not shown again. */
  readonly onMade: (secret: string) => void;
  readonly onCancel: () => void;
}

/** *Create an agent key*, or *Renew*: the password again, and when the key runs out. */
export function KeyForm({ agent, renewing, onMade, onCancel }: KeyFormProps) {
  const rules = useQuery(keyRulesOptions());
  const draft = useKeyDraft();
  const request = useKeyRequest(agent, renewing, onMade);
  if (rules.data === undefined) {
    return <Skeleton />;
  }
  const loaded = rules.data;
  return (
    <Form
      onSubmit={() => {
        request.send(draft, loaded);
      }}
    >
      {renewing === null ? (
        <TextField
          label={t("agents.key-name")}
          description={t("agents.key-name-help")}
          requiredMessage={t("error.field.required")}
          value={draft.name}
          onChange={draft.setName}
        />
      ) : (
        <p {...partOf(part.text)}>{t("agents.key-renew-text", { name: renewing.name })}</p>
      )}
      <KeyExpiry draft={draft} rules={loaded} wrongDay={request.wrongDay} />
      <TextField
        label={t("field.confirm-password")}
        description={t("field.confirm-password-help")}
        kind={fieldKind.password}
        autoComplete={autoComplete.currentPassword}
        requiredMessage={t("error.field.required")}
        value={draft.password}
        onChange={draft.setPassword}
      />
      {request.failure === null ? null : (
        <Alert tone={alertTone.danger}>{messageOf(request.failure)}</Alert>
      )}
      <div {...partOf(part.row, { variants: ["stretch"] })}>
        <SubmitButton isPending={request.pending}>
          {renewing === null ? t("agents.key-create-confirm") : t("agents.key-renew-confirm")}
        </SubmitButton>
        <Button onPress={onCancel}>{t("common.cancel")}</Button>
      </div>
    </Form>
  );
}
