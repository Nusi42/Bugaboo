import { t } from "@bugaboo/messages";

/** What the settings of a project say about *Agents in this project*, as the choice shows it. */
export interface ProjectAgentsValue {
  /** The project has a value of its own. */
  readonly own: boolean;
  /** The value that counts in the project. */
  readonly on: boolean;
  /** Where the value that counts comes from; the short name of the project above when it is one. */
  readonly from: { readonly kind: string; readonly project?: string | undefined };
}

/** The choice of the switch. */
export const agentsChoice = { follow: "follow", off: "off", on: "on" } as const;

/**
 * Which choice stands and how the first one reads: *As the organization (on)*, or *As WEB (off)* when
 * the project above holds the value. A project with a value of its own cannot tell what the level
 * above gives, so the first choice then says no value; an own *on* is a choice of its own, never
 * shown as *As the organization*; so is the way to switch them on below a level that is off.
 */
export function projectAgentsChoice({ own, on, from }: ProjectAgentsValue): {
  readonly selected: (typeof agentsChoice)[keyof typeof agentsChoice];
  readonly follows: string;
  readonly hasOn: boolean;
} {
  if (own) {
    return {
      selected: on ? agentsChoice.on : agentsChoice.off,
      follows: t("setting.agents.project-follows"),
      hasOn: on,
    };
  }
  const project = from.kind === "project" ? from.project : undefined;
  let follows: string;
  if (project === undefined) {
    follows = t(on ? "setting.agents.project-follows-on" : "setting.agents.project-follows-off");
  } else {
    follows = t(
      on
        ? "setting.agents.project-follows-project-on"
        : "setting.agents.project-follows-project-off",
      { project },
    );
  }
  // A project below a level that is off can switch its agents on again.
  return { selected: agentsChoice.follow, follows, hasOn: !on };
}
