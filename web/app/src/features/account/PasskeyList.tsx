import { t } from "@bugaboo/messages";
import type { Schema } from "@bugaboo/api-client";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
import { RemoveWithPassword } from "./RemoveWithPassword.tsx";
import { refreshTwoStep } from "./two-step.ts";

type Passkey = Schema<"PasskeyBody">;

interface PasskeyRowProps {
  readonly passkey: Passkey;
  readonly onRemoved: () => void;
}

/** One passkey: its name, when it was added and used, and *Remove*. */
function PasskeyRow({ passkey, onRemoved }: PasskeyRowProps) {
  const remove = api.query.useMutation("post", "/api/v1/auth/passkeys/{passkey}/remove", {
    onSuccess: () => {
      refreshTwoStep();
      onRemoved();
    },
  });
  const used = passkey.last_used_at;
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.itemText)}>
        <span {...partOf(part.itemTitle)}>{passkey.label}</span>
        <span {...partOf(part.muted)}>
          {t("passkeys.added-on", { when: formatDateTime(passkey.created_at) })}
        </span>
        <span {...partOf(part.muted)}>
          {used === null || used === undefined
            ? t("passkeys.never-used")
            : t("passkeys.last-used", { when: formatDateTime(used) })}
        </span>
        {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
      </span>
      <RemoveWithPassword
        triggerLabel={t("passkeys.remove")}
        triggerName={t("passkeys.remove-name", { name: passkey.label })}
        title={t("passkeys.remove-title", { name: passkey.label })}
        text={t("passkeys.remove-text")}
        confirmLabel={t("passkeys.remove")}
        cancelLabel={t("common.cancel")}
        onConfirm={(password) => {
          remove.mutate({
            params: { path: { passkey: passkey.id } },
            body: { password },
          });
        }}
      />
    </li>
  );
}

export interface PasskeyListProps {
  readonly passkeys: readonly Passkey[];
  /** A passkey was removed. */
  readonly onRemoved: () => void;
}

/** The passkeys of the person, or a sentence that there are none yet. */
export function PasskeyList({ passkeys, onRemoved }: PasskeyListProps) {
  if (passkeys.length === 0) {
    return <p {...partOf(part.muted)}>{t("passkeys.none")}</p>;
  }
  return (
    <ul {...partOf(part.list)} aria-label={t("passkeys.list")}>
      {passkeys.map((passkey) => (
        <PasskeyRow key={passkey.id} passkey={passkey} onRemoved={onRemoved} />
      ))}
    </ul>
  );
}
