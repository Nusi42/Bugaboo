import { reasonMax } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  fieldKind,
  Form,
  SubmitButton,
  TextField,
  Window,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { ConfirmPasswordField } from "../account/index.ts";
import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { useAskSomeonesData } from "./data-exports.ts";
import { type Picked, PersonPicker } from "./SomeonesDataPicker.tsx";

/** What the window asks: the person and the reason, until the server has taken the request. */
function useSomeonesDataForm(onClose: () => void) {
  const [picked, setPicked] = useState<Picked>();
  const [reason, setReason] = useState("");
  const [password, setPassword] = useState("");
  const [unpicked, setUnpicked] = useState(false);
  const ask = useAskSomeonesData();
  return {
    picked,
    reason,
    setReason,
    password,
    setPassword,
    unpicked,
    ask,
    pick: (person: Picked): void => {
      setPicked(person);
      setUnpicked(false);
    },
    submit: (): void => {
      if (picked === undefined) {
        setUnpicked(true);
        return;
      }
      ask.mutate({ person: picked.id, reason: reason.trim(), password }, { onSuccess: onClose });
    },
  };
}

/**
 * *Download someone's data* (PRIV-03, ADR-0044, decision 4): a person picked by name and the reason, always; the
 * person is told, and the security log records who made the file of whose data and why. The window closes once the
 * server has taken the request, and the new file stands at the top of the list.
 */
export function SomeonesDataWindow({ onClose }: { readonly onClose: () => void }) {
  const formId = useId();
  const windowProps = useWindowProps(windowForms.someonesData);
  const form = useSomeonesDataForm(onClose);
  return (
    <Window
      {...windowProps}
      title={t("data-exports.someone")}
      isOpen
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isPending={form.ask.isPending}>
            {t("data-exports.create")}
          </SubmitButton>
        </div>
      }
    >
      <Form id={formId} onSubmit={form.submit}>
        <p {...partOf(part.text)}>{t("data-exports.window-intro")}</p>
        {form.ask.isError ? (
          <Alert tone={alertTone.danger}>{messageOf(form.ask.error)}</Alert>
        ) : null}
        <PersonPicker picked={form.picked} onPick={form.pick} />
        {form.unpicked ? (
          <Alert tone={alertTone.danger}>{t("data-exports.pick-first")}</Alert>
        ) : null}
        <TextField
          label={t("data-exports.reason")}
          description={t("data-exports.reason-help")}
          kind={fieldKind.multiline}
          requiredMessage={t("error.field.required")}
          maxLength={reasonMax}
          value={form.reason}
          onChange={form.setReason}
        />
        <ConfirmPasswordField value={form.password} onChange={form.setPassword} />
      </Form>
    </Window>
  );
}
