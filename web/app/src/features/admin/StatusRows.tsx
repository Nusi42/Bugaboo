import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { daysUntil, formatDate, formatDateTime, splitUptime } from "../../shared/format.ts";
import { LogStatus } from "./LogStatus.tsx";
import {
  CERTIFICATE_WARNING_DAYS,
  openHostParts,
  openParts,
  serverNow,
  unknownHostParts,
  unknownParts,
} from "./status.ts";

type State = Schema<"ServerStatusBody">;

const databaseKinds: Readonly<Record<State["database"]["kind"], MessageKey>> = {
  "built-in": "server-status.database.built-in",
  sqlite: "server-status.database.sqlite",
  postgres: "server-status.database.postgres",
};

/** One line of the status: what it is, and what it says. */
function Fact({ term, children }: { readonly term: string; readonly children: ReactNode }) {
  return (
    <div {...partOf(part.fact)}>
      <dt {...partOf(part.term)}>{term}</dt>
      <dd {...partOf(part.adminDetail)}>{children}</dd>
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

const hostPartNames: Readonly<Record<keyof State["host_open"], MessageKey>> = {
  program: "host.part.program",
  settings_file: "host.part.settings-file",
  tls_key: "host.part.tls-key",
};

/** Parts under a sentence that says why they are listed. */
function Parts<K extends string>({
  parts,
  names,
  heading,
}: {
  readonly parts: readonly K[];
  readonly names: Readonly<Record<K, MessageKey>>;
  readonly heading: string;
}) {
  return (
    <>
      <Alert tone={alertTone.warning}>{heading}</Alert>
      <ul {...partOf(part.list)}>
        {parts.map((piece) => {
          const name: MessageKey = names[piece];
          return <li key={piece}>{t(name)}</li>;
        })}
      </ul>
    </>
  );
}

interface RightsProps<K extends string> {
  readonly opened: readonly K[];
  readonly unknown: readonly K[];
  readonly names: Readonly<Record<K, MessageKey>>;
  readonly closed: string;
  readonly open: string;
}

/** What other accounts on this computer can open or change, or that nothing is. */
function Rights<K extends string>({ opened, unknown, names, closed, open }: RightsProps<K>) {
  if (opened.length === 0 && unknown.length === 0) {
    return <span>{closed}</span>;
  }
  return (
    <>
      {opened.length === 0 ? null : <Parts parts={opened} names={names} heading={open} />}
      {unknown.length === 0 ? null : (
        <Parts parts={unknown} names={names} heading={t("server-status.data-folder.unknown")} />
      )}
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
    return <span>{t("server-status.certificate.none")}</span>;
  }
  const date = formatDate(certificate.expires_at);
  const days = daysUntil(certificate.expires_at, nowMs);
  if (days < 0) {
    return (
      <Alert tone={alertTone.danger}>{t("server-status.certificate.expired", { date })}</Alert>
    );
  }
  if (days < CERTIFICATE_WARNING_DAYS) {
    return (
      <Alert tone={alertTone.warning}>{t("server-status.certificate.soon", { date, days })}</Alert>
    );
  }
  return <span>{t("server-status.certificate.valid", { date, days })}</span>;
}

/** The facts of the server: version, running time, database, certificate and security log. */
export function StatusRows({ state }: { readonly state: State }) {
  return (
    <dl {...partOf(part.facts)}>
      <Fact term={t("server-status.version")}>{state.version}</Fact>
      <Fact term={t("server-status.running")}>
        <span>{t("server-status.uptime", splitUptime(state.uptime_seconds))}</span>
        <span {...partOf(part.muted)}>
          {t("server-status.started", { when: formatDateTime(state.started_at) })}
        </span>
      </Fact>
      <Fact term={t("server-status.database")}>
        <span>{t(databaseKinds[state.database.kind])}</span>
        {state.database.ok ? (
          <span>{t("server-status.database.ok")}</span>
        ) : (
          <Alert tone={alertTone.danger}>{t("server-status.database.failed")}</Alert>
        )}
        {state.database.certificate_unchecked ? (
          <Alert tone={alertTone.warning}>
            {t("server-status.database.certificate-unchecked")}
          </Alert>
        ) : null}
      </Fact>
      <Fact term={t("server-status.certificate")}>
        <Certificate certificate={state.certificate} nowMs={serverNow(state)} />
      </Fact>
      <Fact term={t("server-status.log")}>
        <LogStatus log={state.security_log} />
      </Fact>
      <Fact term={t("server-status.data-folder")}>
        <Rights
          opened={openParts(state.data_folder_open)}
          unknown={unknownParts(state.data_folder_open)}
          names={partNames}
          closed={t("server-status.data-folder.closed")}
          open={t("server-status.data-folder.open")}
        />
      </Fact>
      <Fact term={t("server-status.host")}>
        <Rights
          opened={openHostParts(state.host_open)}
          unknown={unknownHostParts(state.host_open)}
          names={hostPartNames}
          closed={t("server-status.host.closed")}
          open={t("server-status.host.open")}
        />
      </Fact>
    </dl>
  );
}
