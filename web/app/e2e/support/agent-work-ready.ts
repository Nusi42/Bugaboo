import { signedInAdmin } from "./access.ts";
import { createAgent, handOverOne, putScene } from "./agent-work.ts";
import type { ApiSession } from "./api.ts";
import { createIssue } from "./issues.ts";
import { createAppSession } from "./runs.ts";
import type { Instance } from "./server.ts";

/** A new issue handed over to a new agent, with an attempt that is ready for review, one link and one check. */
export async function readyForReview(
  server: Instance,
  title: string,
  name: string,
  withRun: boolean,
): Promise<{
  readonly admin: ApiSession;
  readonly issue: { readonly key: string; readonly id: string };
}> {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", title);
  const agent = await createAgent(admin, name);
  const work = await handOverOne(admin, agent, issue);
  const run = withRun ? await createAppSession(server, admin, [{ issue, version: 1 }]) : undefined;
  putScene(server, work, {
    state: "ready-for-review",
    attempt: {
      plan: "Fix it",
      ...(run === undefined ? {} : { run }),
      results: [
        { kind: "link", label: "Pull request 91", link: "https://example.test/pull/91" },
        { kind: "check", label: "Tests", outcome: "passed" },
      ],
    },
  });
  return { admin, issue };
}
