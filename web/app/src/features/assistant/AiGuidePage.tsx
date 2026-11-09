import { t } from "@bugaboo/messages";
import { Link, MarkdownView, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";

/**
 * The guide to AI for everybody (COMP-11, M7-F90): what the assistant, AI apps and agents can and
 * cannot do, where data goes and how to check what an AI wrote. The server renders the text of the
 * user docs once; the page only shows it.
 */
export function AiGuidePage() {
  usePageTitle(t("assistant.guide-title"));
  const guide = useQuery(api.query.queryOptions("get", "/api/v1/help/ai-guide"));
  return (
    <div {...partOf(part.stack)}>
      <QueryGate error={guide.error} data={guide.data}>
        {(shown) => (
          <>
            <MarkdownView html={shown.html} />
            <Link href={paths.connectedApps}>{t("assistant.guide-apps-link")}</Link>
          </>
        )}
      </QueryGate>
    </div>
  );
}
