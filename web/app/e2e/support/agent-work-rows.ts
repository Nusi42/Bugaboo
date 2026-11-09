import { signedInAdmin } from "./access.ts";
import { createAgent, handOverOne, putScene } from "./agent-work.ts";
import { createIssue } from "./issues.ts";
import { changeStatus } from "./notifications.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";

/** Three issues of the three status groups, each with an agent at work, the keys in that order. */
export async function rowsOfStatuses(server: Instance): Promise<readonly string[]> {
  const admin = await signedInAdmin(server);
  const keys: string[] = [];
  await inOrder(
    (
      [
        ["Row not started", "open"],
        ["Row in progress", "in-progress"],
        ["Row done", "done"],
      ] as const
    ).map(([title, status]) => async () => {
      const issue = await createIssue(admin, "WEB", title);
      const agent = await createAgent(admin, `Agent of ${title}`);
      putScene(server, await handOverOne(admin, agent, issue), { state: "agent-working" });
      if (status !== "open") {
        await changeStatus(admin, issue.key, status);
      }
      keys.push(issue.key);
    }),
  );
  return keys;
}
