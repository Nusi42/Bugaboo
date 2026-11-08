import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  fieldKind,
  Form,
  SelectField,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useChangeAi, useSetAiKey, useTryProvider, type AiSetup } from "./ai.ts";
import { useSetupDraft, type SetupDraft } from "./ai-draft.ts";

/** The two kinds of provider, in the words of the catalog. */
function kindChoices(): readonly { readonly value: AiSetup["provider"]; readonly label: string }[] {
  return [
    { value: "anthropic", label: t("ai.provider.anthropic") },
    { value: "openai-compatible", label: t("ai.provider.openai-compatible") },
  ];
}

type Setter = ReturnType<typeof useSetupDraft>["set"];

/** The provider, the model and, for a model of your own, its address. */
function ProviderFields({ draft, set }: { readonly draft: SetupDraft; readonly set: Setter }) {
  return (
    <>
      <SelectField
        label={t("setting.ai.provider.name")}
        description={t("setting.ai.provider.help")}
        choices={kindChoices()}
        value={draft.kind}
        onChange={set("kind")}
      />
      <TextField
        label={t("setting.ai.model.name")}
        description={t("setting.ai.model.help")}
        value={draft.model}
        onChange={set("model")}
      />
      {draft.kind === "openai-compatible" ? (
        <TextField
          label={t("setting.ai.address.name")}
          description={t("setting.ai.address.help")}
          value={draft.address}
          onChange={set("address")}
        />
      ) : null}
    </>
  );
}

/** The key, which is never shown, and the password that these changes ask for. */
function SecretFields({
  draft,
  set,
  keySet,
}: {
  readonly draft: SetupDraft;
  readonly set: Setter;
  readonly keySet: boolean;
}) {
  return (
    <>
      <TextField
        label={t("ai-setup.key")}
        description={keySet ? t("ai-setup.key-set") : t("ai-setup.key-none")}
        kind={fieldKind.password}
        value={draft.key}
        onChange={set("key")}
      />
      <TextField
        label={t("ai-setup.password")}
        description={t("ai-setup.password-help")}
        kind={fieldKind.password}
        requiredMessage={t("ai-setup.password-required")}
        value={draft.password}
        onChange={set("password")}
      />
    </>
  );
}

/** What *Try the provider* answered: the provider answered, or why it did not. */
function TryAnswer({ done, error }: { readonly done: boolean; readonly error: unknown }) {
  if (done) {
    return <Alert tone={alertTone.success}>{t("ai-setup.try-done")}</Alert>;
  }
  return error === null || error === undefined ? null : (
    <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
  );
}

/** Saves the setup, then the key when one was typed; the key is stored encrypted and never read back. */
function useSave(draft: SetupDraft, forget: () => void) {
  const change = useChangeAi();
  const key = useSetAiKey();
  const save = (): void => {
    const own = draft.kind === "openai-compatible";
    const body = {
      provider: draft.kind,
      model: draft.model.trim(),
      address: own ? draft.address.trim() : "",
      password: draft.password,
    };
    change.mutate(
      { body },
      {
        onSuccess: () => {
          if (draft.key !== "") {
            key.mutate(
              { body: { key: draft.key, password: draft.password } },
              { onSuccess: forget },
            );
          }
        },
      },
    );
  };
  return { change, key, save };
}

/**
 * *Set up the assistant* (AI-01, PRIV-08): the provider, the model, the address of a model of your
 * own and the key. Changing the provider, the address or the key asks for the password again.
 * *Try the provider* asks the provider one short question that holds no data of the organization.
 */
export function AiSetupForm({ setup }: { readonly setup: AiSetup }) {
  const { draft, set } = useSetupDraft(setup);
  const { change, key, save } = useSave(draft, () => {
    set("key")("");
  });
  const attempt = useTryProvider();
  return (
    <Form onSubmit={save}>
      <ProviderFields draft={draft} set={set} />
      <SecretFields draft={draft} set={set} keySet={setup.key_set} />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={change.isPending || key.isPending}>
          {t("ai-setup.save")}
        </SubmitButton>
        <Button
          look={buttonLook.plain}
          isPending={attempt.isPending}
          onPress={() => {
            attempt.mutate({});
          }}
        >
          {t("ai-setup.try")}
        </Button>
        {setup.key_set ? (
          <Button
            look={buttonLook.quiet}
            isDisabled={draft.password.length === 0}
            onPress={() => {
              key.mutate({ body: { password: draft.password } });
            }}
          >
            {t("ai-setup.key-remove")}
          </Button>
        ) : null}
      </div>
      {change.isSuccess ? <Alert tone={alertTone.success}>{t("ai-setup.saved")}</Alert> : null}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
      {key.isError ? <Alert tone={alertTone.danger}>{messageOf(key.error)}</Alert> : null}
      <TryAnswer done={attempt.isSuccess} error={attempt.error} />
    </Form>
  );
}
