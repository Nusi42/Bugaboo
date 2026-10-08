import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, type AlertTone, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { type Level, levelOf, serverNow } from "./status.ts";
import { StatusRows } from "./StatusRows.tsx";

type State = Schema<"ServerStatusBody">;

const levelMessages: Readonly<Record<Level, MessageKey>> = {
  ok: "status.state.ok",
  warning: "status.state.warning",
  problem: "status.state.problem",
};

const levelTones: Readonly<Record<Level, AlertTone>> = {
  ok: alertTone.success,
  warning: alertTone.warning,
  problem: alertTone.danger,
};

/** The words for the level, in an alert that says it all in one line. */
function Summary({ state }: { readonly state: State }) {
  const level = levelOf(state, serverNow(state));
  return <Alert tone={levelTones[level]}>{t(levelMessages[level])}</Alert>;
}

/** The status page for administrators: how the server is doing (OPS-08). */
export function StatusPage() {
  usePageTitle(t("status.title"));
  const status = useQuery(api.query.queryOptions("get", "/api/v1/admin/status"));
  return (
    <Card heading={t("status.title")} level={1}>
      <p {...partOf(part.text)}>{t("status.intro")}</p>
      {status.isError ? <Alert tone={alertTone.danger}>{messageOf(status.error)}</Alert> : null}
      {status.data === undefined ? null : <Summary state={status.data} />}
      {status.data === undefined ? null : <StatusRows state={status.data} />}
      <div {...partOf(part.row, { variants: ["stretch"] })}>
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
