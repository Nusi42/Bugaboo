import { dataOf } from "@bugaboo/api-client";
import { expect } from "@playwright/test";

import { tool } from "../../../tooling/load/agents.ts";
import { ada } from "./accounts.ts";
import { signedInAdmin } from "./access.ts";
import { createAgent } from "./agents-scene.ts";
import { type ApiSession } from "./api.ts";
import { createIssue } from "./issues.ts";
import { type Instance } from "./server.ts";

/** The project the issue of each scene is made in. */
const HISTORY_PROJECT = "WEB";

/** The comment the agent writes in its run, which its undo throws away. */
export const AGENT_COMMENT = "Looked at it, nothing to change";

/** How many scenes this server has had: agent names are taken for good. */
let scenesMade = 0;

/** Two agents of the administrator: Fixer, with a key and the issue of the scene handed over to it, and Helper. */
export interface HistoryScene {
  readonly admin: ApiSession;
  /** The readable ID of the issue of this scene, made for it alone, so scenes that run at once do not meet. */
  readonly issue: string;
  readonly names: { readonly fixer: string; readonly helper: string };
  readonly fixer: string;
  readonly key: string;
  /** Stops every agent of the administrator and signs out. */
  readonly dispose: () => Promise<void>;
}

/** Fixer and Helper of the administrator, and an issue of their own; Fixer has a key and the issue handed over to it. */
export async function historyScene(server: Instance): Promise<HistoryScene> {
  const admin = await signedInAdmin(server);
  scenesMade += 1;
  const suffix = ` ${String(scenesMade)}`;
  const names = { fixer: `Fixer${suffix}`, helper: `Helper${suffix}` };
  const issue = (await createIssue(admin, HISTORY_PROJECT, `The history of an agent${suffix}`)).key;
  const fixer = await createAgent(admin, names.fixer);
  await createAgent(admin, names.helper);
  const key = dataOf(
    await admin.client.POST("/api/v1/agents/{agent}/keys", {
      params: { path: { agent: fixer } },
      body: { name: "Key", password: ada.password },
    }),
  ).secret;
  dataOf(
    await admin.client.POST("/api/v1/hand-overs", {
      body: {
        agent: fixer,
        issues: [issue],
        note: "Please look at it",
        powers: [],
        fields: [],
        plan_needs_approval: false,
        accept_by_itself: false,
      },
    }),
  );
  const dispose = async (): Promise<void> => {
    await admin.client.POST("/api/v1/me/agents/stop-all");
    await admin.dispose();
  };
  return { admin, issue, names, fixer, key: key ?? "", dispose };
}

/** Switches both switches of the agent on, as its person does on its page. */
export async function bothSwitchesOn(scene: HistoryScene): Promise<void> {
  const agent = dataOf(
    await scene.admin.client.GET("/api/v1/agents/{agent}", {
      params: { path: { agent: scene.fixer } },
    }),
  );
  dataOf(
    await scene.admin.client.PATCH("/api/v1/agents/{agent}", {
      params: { path: { agent: scene.fixer } },
      body: { switches: { may_hand_over: true, may_undo: true }, version: agent.version ?? 1 },
    }),
  );
}

/** Fixer takes the issue, comments, undoes its run and hands the issue over to Helper, through its tools. */
export async function fixerUndoesAndHandsOver(
  server: Instance,
  scene: HistoryScene,
): Promise<void> {
  const does = async (name: string, args: Readonly<Record<string, unknown>>) => {
    expect((await tool(server, scene.key, name, args)).kept, `${name} of the agent`).toBe(true);
  };
  await does("take", { issue: scene.issue });
  await does("add_comment", { issue: scene.issue, text: AGENT_COMMENT });
  await does("undo_my_run", { issue: scene.issue });
  await does("hand_over_work", { agent: scene.names.helper, issues: [scene.issue] });
}
