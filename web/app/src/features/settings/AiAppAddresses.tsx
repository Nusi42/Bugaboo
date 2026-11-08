import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  buttonLook,
  ConfirmDialog,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { formatDate } from "../../shared/format.ts";
import { useAddAppAddress, useAppAddresses, useRemoveAppAddress } from "./ai.ts";

/** The form that allows one more web address. */
function AddAddress() {
  const [address, setAddress] = useState("");
  const add = useAddAppAddress();
  return (
    <Form
      onSubmit={() => {
        add.mutate(
          { body: { address: address.trim() } },
          {
            onSuccess: () => {
              setAddress("");
            },
          },
        );
      }}
    >
      <TextField
        label={t("ai-setup.address-new")}
        description={t("ai-setup.address-help")}
        requiredMessage={t("ai-setup.address-required")}
        value={address}
        onChange={setAddress}
      />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={add.isPending}>{t("ai-setup.address-add")}</SubmitButton>
      </div>
      {add.isError ? <Alert tone={alertTone.danger}>{messageOf(add.error)}</Alert> : null}
    </Form>
  );
}

/**
 * *Apps that may sign in* (AI-22, M7-Q4): the web addresses an administrator allows for AI apps that
 * run on the web. An app on a person's own computer needs none. Taking an address away refuses the
 * apps that registered with it at their next sign-in.
 */
export function AiAppAddresses() {
  const found = useAppAddresses();
  const remove = useRemoveAppAddress();
  const items = found.data?.items ?? [];
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("ai-setup.addresses-intro")}</p>
      {items.length === 0 ? (
        <p {...partOf(part.muted)}>{t("ai-setup.addresses-none")}</p>
      ) : (
        <ul {...partOf(part.list)} aria-label={t("ai-setup.addresses-list")}>
          {items.map((each) => (
            <li key={each.address} {...partOf(part.item)}>
              <span {...partOf(part.itemText)}>
                <span {...partOf(part.itemTitle)}>{each.address}</span>
                {each.added_at === undefined || each.added_at === null ? null : (
                  <span {...partOf(part.muted)}>
                    {t("ai-setup.address-added", { date: formatDate(each.added_at) })}
                  </span>
                )}
              </span>
              <ConfirmDialog
                triggerLabel={t("ai-setup.address-remove")}
                triggerName={t("ai-setup.address-remove-name", { address: each.address ?? "" })}
                title={t("ai-setup.address-remove-title")}
                text={t("ai-setup.address-remove-text")}
                confirmLabel={t("ai-setup.address-remove")}
                cancelLabel={t("common.cancel")}
                confirmLook={buttonLook.danger}
                onConfirm={() => {
                  remove.mutate({ body: { address: each.address ?? "" } });
                }}
              />
            </li>
          ))}
        </ul>
      )}
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
      <AddAddress />
    </div>
  );
}
