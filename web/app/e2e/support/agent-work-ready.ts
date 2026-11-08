import { signedInAdmin } from "./access.ts";
import { ownId } from "./accounts.ts";
import {
  createAgent,
  handOverOne,
  putScene,
  type HandOverAsk,
  type WorkScene,
} from "./agent-work.ts";
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
  before: Pick<WorkScene, "earlier" | "review"> = {},
): Promise<{
  readonly admin: ApiSession;
  readonly issue: { readonly key: string; readonly id: string };
}> {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", title, "A description");
  const agent = await createAgent(admin, name);
  const work = await handOverOne(admin, agent, issue);
  const run = withRun ? await createAppSession(server, admin, [{ issue, version: 1 }]) : undefined;
  const attempt = {
    number: (before.earlier?.length ?? 0) + 1,
    plan: "1. Find the check\n2. Cut the lifetime\n3. Add a test",
    minutes: 25,
    cost: 40,
    // An attempt that ended with a result has an end.
    ended: true,
    results: [
      { kind: "link", label: "Pull request 91", link: "https://example.test/pull/91" },
      { kind: "check", label: "Tests", outcome: "passed" },
    ],
    ...(run === undefined ? {} : { run }),
  } as const;
  putScene(server, work, { ...before, state: "ready-for-review", attempt }, await ownId(admin));
  return { admin, issue };
}

/** A new issue with a description, handed to a new agent, whose work is set to the scene. */
export async function sceneIssue(
  server: Instance,
  title: string,
  name: string,
  scene: WorkScene,
  ask: HandOverAsk = {},
): Promise<{
  readonly admin: ApiSession;
  readonly issue: { readonly key: string; readonly id: string };
}> {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", title, "A description");
  const agent = await createAgent(admin, name);
  const work = await handOverOne(admin, agent, issue, ask);
  putScene(server, work, scene, await ownId(admin));
  return { admin, issue };
}
