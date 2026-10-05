import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { daysUntil, formatDate, formatDateTime, splitUptime } from "../../shared/format.ts";
import styles from "./Admin.module.css";
import { LogStatus } from "./LogStatus.tsx";
import { CERTIFICATE_WARNING_DAYS, openParts, serverNow } from "./status.ts";

type State = Schema<"ServerStatusBody">;

const databaseKinds: Readonly<Record<State["database"]["kind"], MessageKey>> = {
  built_in: "status.database.built-in",
  sqlite: "status.database.sqlite",
  postgres: "status.database.postgres",
};

/** One line of the status: what it is, and what it says. */
function Fact({ term, children }: { readonly term: string; readonly children: ReactNode }) {
  return (
    <div className={styles.fact}>
      <dt className={styles.term}>{term}</dt>
      <dd className={styles.detail}>{children}</dd>
    </div>
  );
}

const partNames: Readonly<Record<keyof State["data_folder_open"], MessageKey>> = {
  folder: "data-folder.part.folder",
  database: "data-folder.part.database",
  files: "data-folder.part.files",
  backups: "data-folder.part.backups",
  setup_code_file: "data-folder.part.setup-code-file",
};

/** What of the data folder other accounts on this computer can open, or that nothing is. */
function DataFolderRights({ open }: { readonly open: State["data_folder_open"] }) {
  const parts = openParts(open);
  if (parts.length === 0) {
    return <span>{t("status.data-folder.closed")}</span>;
  }
  return (
    <>
      <Alert warning>{t("status.data-folder.open")}</Alert>
      <ul className={styles.list}>
        {parts.map((part) => (
          <li key={part}>{t(partNames[part])}</li>
        ))}
      </ul>
    </>
  );
}

interface CertificateProps {
  readonly certificate: State["certificate"];
  readonly nowMs: number;
}

/** The state of the HTTPS certificate: how long it is valid, in words. */
function Certificate({ certificate, nowMs }: CertificateProps) {
  if (certificate === null || certificate === undefined) {
    return <span>{t("status.certificate.none")}</span>;
  }
  const date = formatDate(certificate.expires_at);
  const days = daysUntil(certificate.expires_at, nowMs);
  if (days < 0) {
    return <Alert danger>{t("status.certificate.expired", { date })}</Alert>;
  }
  if (days < CERTIFICATE_WARNING_DAYS) {
    return <Alert warning>{t("status.certificate.soon", { date, days })}</Alert>;
  }
  return <span>{t("status.certificate.valid", { date, days })}</span>;
}

/** The facts of the server: version, running time, database, certificate and security log. */
export function StatusRows({ state }: { readonly state: State }) {
  return (
    <dl className={styles.facts}>
      <Fact term={t("status.version")}>{state.version}</Fact>
      <Fact term={t("status.running")}>
        <span>{t("status.uptime", splitUptime(state.uptime_seconds))}</span>
        <span className={styles.muted}>
          {t("status.started", { when: formatDateTime(state.started_at) })}
        </span>
      </Fact>
      <Fact term={t("status.database")}>
        <span>{t(databaseKinds[state.database.kind])}</span>
        {state.database.ok ? (
          <span>{t("status.database.ok")}</span>
        ) : (
          <Alert danger>{t("status.database.failed")}</Alert>
        )}
      </Fact>
      <Fact term={t("status.certificate")}>
        <Certificate certificate={state.certificate} nowMs={serverNow(state)} />
      </Fact>
      <Fact term={t("status.log")}>
        <LogStatus log={state.security_log} />
      </Fact>
      <Fact term={t("status.data-folder")}>
        <DataFolderRights open={state.data_folder_open} />
      </Fact>
    </dl>
  );
}
