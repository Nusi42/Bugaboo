import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, keepsRoom, partOf, part, SaveStatus } from "@bugaboo/ui";
import { useQueryClient } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { type Asked, backupsOptions, createState } from "./backups.ts";

/** Why the backup failed, in words: the refusal of the ask, or the backup the server could not make. */
function Failure({ asked, refusal }: { readonly asked: Asked | null; readonly refusal: unknown }) {
  if (refusal !== null) {
    return <Alert tone={alertTone.danger}>{messageOf(refusal)}</Alert>;
  }
  return asked?.status === "failed" ? (
    <Alert tone={alertTone.danger}>{t("backup-page.failed")}</Alert>
  ) : null;
}

/**
 * *Create a backup now*: the server makes the backup in the background, so the button waits in place and says so
 * beside itself while the list says it runs (`asked`), and the new row comes in when the backup is there.
 */
export function CreateBackup({ asked }: { readonly asked: Asked | null }) {
  const client = useQueryClient();
  const create = api.query.useMutation("post", "/api/v1/admin/backups", {
    // The ask waits for the list, so the words go from sending to running without a gap.
    onSuccess: () => client.invalidateQueries({ queryKey: backupsOptions().queryKey }),
  });
  const state = createState(asked, create);
  const busy = state === "saving";
  return (
    <div {...partOf(part.stack)}>
      <div {...partOf(part.row)}>
        <Button
          isPending={busy}
          onPress={() => {
            create.mutate({});
          }}
        >
          {t("backup-page.create")}
        </Button>
        <SaveStatus
          state={state}
          labels={{
            saving: t("backup-page.creating"),
            saved: t("backup-page.created"),
            "not-saved": t("backup-page.not-created"),
          }}
          variant={keepsRoom}
        />
      </div>
      <p {...partOf(part.muted)}>{t("backup-page.create-help")}</p>
      <Failure asked={asked} refusal={create.isError ? create.error : null} />
    </div>
  );
}
