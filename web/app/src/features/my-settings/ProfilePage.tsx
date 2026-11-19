import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Button,
  Card,
  fieldKind,
  Form,
  InPlaceText,
  keepsRoom,
  SaveStatus,
  SubmitButton,
  TextField,
  partOf,
  part,
  type SaveState,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { codeOf, messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import { keepPersonName } from "../../shared/session.ts";
import { useMe } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";

/**
 * A change of the name: the name after it, and the name *Undo* goes back to. None for the change
 * that *Undo* makes, which offers no *Undo* of its own.
 */
interface NameChange {
  readonly name: string;
  readonly before?: string;
}

/**
 * The name, changed in place: it saves at once, says *Saved* beside it, and offers *Undo* like every
 * change that can be taken back (UX R7). *Undo* gives back the name the person had before.
 */
function NameRow({ name }: { readonly name: string }) {
  const [state, setState] = useState<SaveState>("idle");
  const change = api.query.useMutation("put", "/api/v1/me/name");
  const save = (made: NameChange, settled?: (saved: boolean) => void) => {
    setState("saving");
    change.mutate(
      { body: { name: made.name } },
      {
        onSuccess: (data) => {
          keepPersonName(data.name);
          setState("saved");
          settled?.(true);
          const before = made.before;
          if (before !== undefined && before !== data.name) {
            undoStore.offer({
              message: t("profile.name-changed"),
              undo: () => {
                save({ name: before });
              },
            });
          }
        },
        onError: () => {
          setState("not-saved");
          settled?.(false);
        },
      },
    );
  };
  return (
    <div {...partOf(part.formSection)}>
      <h2 {...partOf(part.formSectionTitle)}>{t("profile.name")}</h2>
      <InPlaceText
        label={t("profile.name")}
        buttonLabel={(shown) => t("profile.change-name", { name: shown })}
        value={name}
        required
        onSave={(next, settled) => {
          save({ name: next, before: name }, settled);
        }}
      />
      <div {...partOf(part.row)}>
        <SaveStatus state={state} labels={saveLabels()} variant={keepsRoom} />
      </div>
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </div>
  );
}

/** What the server said to a new address, in words: e-mail is off, or another problem. */
function problemWords(error: unknown): string {
  return codeOf(error) === "auth.wrong-credentials"
    ? t("profile.wrong-password")
    : messageOf(error);
}

interface EmailFormProps {
  /** Called with the address a link went to. */
  readonly onSent: (address: string) => void;
  readonly onCancel: () => void;
}

/** The new address and the password, asked again; a link goes to the new address. */
function EmailForm({ onSent, onCancel }: EmailFormProps) {
  const [address, setAddress] = useState("");
  const [password, setPassword] = useState("");
  const ask = api.query.useMutation("post", "/api/v1/me/e-mail-change", {
    onSuccess: () => {
      onSent(address.trim());
    },
  });
  return (
    <Form
      onSubmit={() => {
        ask.mutate({ body: { new_email: address.trim(), password } });
      }}
    >
      <TextField
        label={t("profile.new-e-mail")}
        kind={fieldKind.email}
        autoComplete={autoComplete.email}
        requiredMessage={t("error.field.required")}
        value={address}
        onChange={setAddress}
      />
      <TextField
        label={t("field.confirm-password")}
        description={t("field.confirm-password-help")}
        kind={fieldKind.password}
        autoComplete={autoComplete.currentPassword}
        requiredMessage={t("error.field.required")}
        value={password}
        onChange={setPassword}
      />
      {ask.isError ? <Alert tone={alertTone.danger}>{problemWords(ask.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={ask.isPending}>{t("profile.send-link")}</SubmitButton>
        <Button onPress={onCancel}>{t("common.cancel")}</Button>
      </div>
    </Form>
  );
}

/**
 * The e-mail address and the way to change it: a link goes to the new address, and the old one
 * counts until it is opened.
 */
function EmailRow({ email }: { readonly email: string }) {
  const [open, setOpen] = useState(false);
  const [sentTo, setSentTo] = useState<string | null>(null);
  return (
    <div {...partOf(part.formSection)}>
      <h2 {...partOf(part.formSectionTitle)}>{t("profile.e-mail")}</h2>
      {/* The address is changed through its link, not in place, but stands where a value changed in
          place stands, so the gap under its heading is the one under *Name*. */}
      <span {...partOf(part.inPlaceText)}>{email}</span>
      {sentTo === null ? null : (
        <Alert tone={alertTone.info}>{t("profile.link-sent", { address: sentTo })}</Alert>
      )}
      {open ? (
        <EmailForm
          onSent={(address) => {
            setSentTo(address);
            setOpen(false);
          }}
          onCancel={() => {
            setOpen(false);
          }}
        />
      ) : (
        <div {...partOf(part.row)}>
          <Button
            onPress={() => {
              setOpen(true);
              setSentTo(null);
            }}
          >
            {t("profile.change-e-mail")}
          </Button>
        </div>
      )}
    </div>
  );
}

/**
 * *My settings › Profile* (PRIV-06, ADR-0044, decision 9): the person's name, changed at once with
 * *Undo*, and their e-mail address, changed through a link sent to the new address.
 */
export function ProfilePage() {
  usePageTitle(t("profile.title"));
  const me = useMe();
  return (
    <Card heading={t("profile.title")} level={1} intro={t("profile.intro")}>
      {me === undefined ? null : (
        <>
          <NameRow name={me.name} />
          <EmailRow email={me.email} />
        </>
      )}
    </Card>
  );
}
