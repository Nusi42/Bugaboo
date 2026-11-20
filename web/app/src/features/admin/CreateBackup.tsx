import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, keepsRoom, partOf, part, SaveStatus } from "@bugaboo/ui";
import type { SaveState } from "@bugaboo/ui";
import { useQueryClient } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { backupsOptions } from "./backups.ts";

/** *Create a backup now*: the button waits in place and says so beside itself, and the list reads again when the backup is there. */
export function CreateBackup() {
  const client = useQueryClient();
  const create = api.query.useMutation("post", "/api/v1/admin/backups", {
    // The mutation waits for the list, so the new row comes in with the end of the wait and not a moment later.
    onSuccess: () => client.invalidateQueries({ queryKey: backupsOptions().queryKey }),
  });
  let state: SaveState = "idle";
  if (create.isPending) {
    state = "saving";
  } else if (create.isError) {
    state = "not-saved";
  } else if (create.isSuccess) {
    state = "saved";
  }
  return (
    <div {...partOf(part.stack)}>
      <div {...partOf(part.row)}>
        <Button
          isPending={create.isPending}
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
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
    </div>
  );
}
