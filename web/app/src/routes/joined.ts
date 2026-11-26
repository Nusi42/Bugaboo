/**
 * What joins the shell once somebody is signed in, before their first page: the features that put their parts into the
 * sidebar, quick search, the issue page and the bar. The dynamic imports of this file are exactly the features every
 * signed-in first page waits for; the build puts them and what they need into one file (`tooling/joined-chunk.ts`),
 * so a browser that opens a connection for each file does not fetch dozens of small ones before the first page.
 */

import { loadMySettings } from "../shared/my-settings.ts";

/** Whether the projects have joined the sidebar and quick search. */
let projectsJoined = false;

/**
 * The projects join the sidebar and quick search once somebody is signed in, so the pages before
 * sign-in do not carry them. Only the first time waits: a page that waited on every step would
 * show the next page a moment later, and the focus would have moved to the old one by then.
 */
async function joinProjects(): Promise<void> {
  if (projectsJoined) {
    return;
  }
  const [projects, pins] = await Promise.all([
    import("../features/projects/index.ts"),
    import("../features/pins/index.ts"),
  ]);
  // Marked only now: when the import fails, the next page tries again. Two pages that waited
  // together join once.
  if (!projectsJoined) {
    projectsJoined = true;
    // *Pinned* stands above the project tree, so it joins first.
    pins.registerPins();
    projects.registerProjects();
  }
}

let timeJoined = false;

/** The timer and the work day join the shell once somebody is signed in, like the projects do. */
async function joinTime(): Promise<void> {
  if (timeJoined) {
    return;
  }
  const time = await import("../features/time/index.ts");
  if (!timeJoined) {
    timeJoined = true;
    time.registerTime();
  }
}

let assistantJoined = false;

/** The assistant joins the shell once somebody is signed in; its panel loads when it is first opened. */
async function joinAssistant(): Promise<void> {
  if (assistantJoined) {
    return;
  }
  const assistant = await import("../features/assistant-entry/index.ts");
  if (!assistantJoined) {
    assistantJoined = true;
    assistant.registerAssistant();
  }
}

/** Whether the issues have joined the shell (the new-issue dialog and its key). */
let issuesJoined = false;

/** The issues join the shell once somebody is signed in, like the projects do. */
async function joinIssues(): Promise<void> {
  if (issuesJoined) {
    return;
  }
  const [
    issues,
    fields,
    relations,
    history,
    follow,
    labels,
    manyChanges,
    agentWork,
    agents,
    files,
    search,
    dashboards,
  ] = await Promise.all([
    import("../features/issues/index.ts"),
    import("../features/fields/index.ts"),
    import("../features/relations/index.ts"),
    import("../features/history/index.ts"),
    import("../features/follow/index.ts"),
    import("../features/labels/index.ts"),
    import("../features/many-changes/index.ts"),
    import("../features/agent-work/index.ts"),
    import("../features/agents/index.ts"),
    import("../features/files/index.ts"),
    import("../features/search/index.ts"),
    import("../features/dashboards/index.ts"),
  ]);
  if (!issuesJoined) {
    issuesJoined = true;
    issues.registerIssues();
    // The blocks of the issue page, all put into the slots of `features/issues/slots.ts`.
    fields.registerFields();
    relations.registerRelationBlocks();
    history.registerHistoryBlock();
    follow.registerFollowersBlock();
    labels.registerLabelBlocks();
    manyChanges.registerManyChanges();
    agentWork.registerAgentWork();
    agents.registerAgents();
    files.registerFileBlocks();
    search.registerSearch();
    dashboards.registerDashboards();
  }
}

/**
 * Everything a signed-in page waits for before it is first shown, in this order: the person's own settings, so the look
 * is theirs from the start, then the features that join the shell.
 */
export async function joinTheShell(): Promise<void> {
  await loadMySettings();
  await joinProjects();
  await joinIssues();
  await joinTime();
  await joinAssistant();
}
