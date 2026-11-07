import { ApiError } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Skeleton, partOf, part } from "@bugaboo/ui";
import { useSearch } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ConnectForm } from "./ConnectForm.tsx";
import { connectRequestOf, type ConnectRequest } from "./request.ts";

export interface ConnectPageProps {
  /** What the app put in the address. */
  readonly request: ConnectRequest;
}

/** The plain sentence for a request the server could not take: an app it does not know, or one it cannot answer. */
function refusal(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.problem?.code === "ai.off") {
      return t("connect.off");
    }
    if (error.problem?.code === "request.invalid") {
      return t("connect.invalid");
    }
  }
  return t("connect.unknown");
}

/**
 * *Connect an app* (AI-22, M7-Q3): an AI app wants to work with the account. The page says which app,
 * what it may do and where it comes back to, and asks *Allow* or *Cancel*. The server checks the
 * request again when the person answers, so nothing here decides what the app may do.
 */
export function ConnectPage({ request }: ConnectPageProps) {
  usePageTitle(t("connect.page"));
  const seen = useQuery(
    api.query.queryOptions("get", "/api/v1/oauth/authorization", {
      params: { query: request },
    }),
  );
  return (
    <div {...partOf(part.stack)}>
      <Card
        heading={
          seen.data === undefined
            ? t("connect.page")
            : t("connect.title", { app: seen.data.app_name })
        }
        level={1}
      >
        {seen.isPending ? <Skeleton /> : null}
        {seen.isError ? <Alert tone={alertTone.danger}>{refusal(seen.error)}</Alert> : null}
        {seen.data === undefined ? null : (
          <ConnectForm
            request={request}
            asked={seen.data.asked}
            onThisComputer={seen.data.on_this_computer}
          />
        )}
      </Card>
    </div>
  );
}

/** The page at its address: what the app put in the address is the request. */
export function ConnectScreen() {
  const search: Readonly<Record<string, unknown>> = useSearch({ strict: false });
  return <ConnectPage request={connectRequestOf(search)} />;
}
