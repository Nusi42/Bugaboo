import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link, partOf, part, useHoldWhileReading } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { OrganizationTopic } from "../settings/index.ts";
import { backupRows, backupsOptions, chosenName, RESTORE_DOCS, restoreCommand } from "./backups.ts";
import { BackupsList } from "./BackupsList.tsx";
import { CreateBackup } from "./CreateBackup.tsx";

/** The settings of the schedule: every day on or off, and how many daily and weekly backups are kept. */
const SCHEDULE = "backups.daily,backups.keep-daily,backups.keep-weekly";

/** The word of ARIA for a region whose changes are announced when the person is idle. */
const POLITE = "polite";

/** How a backup is put back: the command for the chosen backup, and where the steps are. There is no button that restores. */
function Restore({ name }: { readonly name: string | undefined }) {
  return (
    <Card heading={t("backup-page.restore")} intro={t("backup-page.restore-intro")}>
      {/* The command changes when another backup is chosen: a polite region says so to a screen reader. */}
      <div aria-live={POLITE}>
        {name === undefined ? null : (
          <code {...partOf(part.keyBox, { variants: ["small"] })}>{restoreCommand(name)}</code>
        )}
      </div>
      <div {...partOf(part.row)}>
        <Link href={RESTORE_DOCS} external>
          {t("backup-page.restore-docs")}
        </Link>
      </div>
    </Card>
  );
}

/** *Organization settings › Backups* (OPS-07): the daily backup, *Create a backup now*, the backups, and how to restore one. */
export function BackupsPage() {
  usePageTitle(t("nav.backups"));
  const backups = useQuery(backupsOptions());
  useHoldWhileReading(backups.isLoading);
  const [picked, setPicked] = useState<string | null>(null);
  const rows = backups.data === undefined ? [] : backupRows(backups.data);
  const chosen = chosenName(rows, picked);
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.backups")} level={1} intro={t("backup-page.intro")} />
      <Card heading={t("backup-page.daily")}>
        <OrganizationTopic topics={SCHEDULE} searchable={false} />
      </Card>
      <Card heading={t("backup-page.list")}>
        <CreateBackup />
        {backups.isError ? <Alert tone={alertTone.danger}>{messageOf(backups.error)}</Alert> : null}
        <BackupsList rows={rows} chosen={chosen} onChoose={setPicked} failed={backups.isError} />
      </Card>
      <Restore name={chosen} />
    </div>
  );
}
