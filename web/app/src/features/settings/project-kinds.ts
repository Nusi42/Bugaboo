import { agentsEnabledKey, aiAppsEnabledKey } from "./agents-switch.tsx";

/**
 * The settings of a project that are the switches of a kind of AI (WP-15): agents, then AI apps. The
 * assistant has a switch of its own on its own route, and is drawn after these two (the order of the
 * three everywhere: agents, AI apps, the assistant last). The keys are written once, in
 * `agents-switch.tsx`: these two settings are declared in `settings/agents.rs` and
 * `settings/ai_apps.rs`, which the generated list `settingKeys` does not read (it reads the files
 * named `catalog.rs` of the registry).
 */
export const projectKindKeys = [agentsEnabledKey, aiAppsEnabledKey] as const;

/** Which settings of a project a section draws: the ones that are not a kind of AI, or only the kinds. */
export const projectShows = { others: "others", kindsOfAi: "kinds-of-ai" } as const;

/** Whether a setting is the switch of a kind of AI. */
function isKind(setting: { readonly key: string }): boolean {
  return (projectKindKeys as readonly string[]).includes(setting.key);
}

/** The switches of the kinds among `settings`, in the order of the kinds. */
export function kindsOf<T extends { readonly key: string }>(settings: readonly T[]): T[] {
  return projectKindKeys.flatMap((key) => settings.filter((setting) => setting.key === key));
}

/** The settings that are not the switch of a kind. */
export function otherThanKinds<T extends { readonly key: string }>(settings: readonly T[]): T[] {
  return settings.filter((setting) => !isKind(setting));
}
