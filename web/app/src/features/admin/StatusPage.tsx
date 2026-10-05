import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Alert, Button, Card } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./Admin.module.css";
import { type Level, levelOf, serverNow } from "./status.ts";
import { StatusRows } from "./StatusRows.tsx";

type State = Schema<"ServerStatusBody">;

const levelMessages: Readonly<Record<Level, MessageKey>> = {
  ok: "status.state.ok",
  warning: "status.state.warning",
  problem: "status.state.problem",
};

/** The words for the level, in an alert that says it all in one line. */
function Summary({ state }: { readonly state: State }) {
  const level = levelOf(state, serverNow(state));
  const ok = level === "ok";
  const problem = level === "problem";
  return (
    <Alert success={ok} warning={!ok && !problem} danger={problem}>
      {t(levelMessages[level])}
    </Alert>
  );
}

/** The status page for administrators: how the server is doing (OPS-08). */
export function StatusPage() {
  usePageTitle(t("status.title"));
  const status = useQuery(api.query.queryOptions("get", "/api/v1/admin/status"));
  return (
    <Card heading={t("status.title")} level={1}>
      <p className={styles.text}>{t("status.intro")}</p>
      {status.isError ? <Alert danger>{messageOf(status.error)}</Alert> : null}
      {status.data === undefined ? null : <Summary state={status.data} />}
      {status.data === undefined ? null : <StatusRows state={status.data} />}
      <div className={styles.row}>
        <Button
          isPending={status.isFetching}
          onPress={() => {
            void status.refetch();
          }}
        >
          {t("status.refresh")}
        </Button>
      </div>
    </Card>
  );
}
