import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { projectAgentsChoice } from "./project-agents-choice.ts";

describe("ai_30 the switch of agents in a project", () => {
  it("shows the value of the organization for a project without a value of its own", () => {
    const shown = projectAgentsChoice({ own: false, on: true, from: { kind: "organization" } });
    expect(shown.selected).toBe("follow");
    expect(shown.follows).toBe(t("setting.agents.project-follows-on"));
    expect(shown.hasOn).toBe(false);
  });

  it("names the project above when the value comes from it", () => {
    const shown = projectAgentsChoice({
      own: false,
      on: false,
      from: { kind: "project", project: "WEB" },
    });
    expect(shown.follows).toBe(t("setting.agents.project-follows-project-off", { project: "WEB" }));
    // Below a level that is off, the project can switch its agents on again.
    expect(shown.hasOn).toBe(true);
  });

  it("shows an own off as off", () => {
    const shown = projectAgentsChoice({ own: true, on: false, from: { kind: "project" } });
    expect(shown.selected).toBe("off");
    expect(shown.follows).toBe(t("setting.agents.project-follows"));
  });

  it("shows an own on as its own value, not as the organization's", () => {
    const shown = projectAgentsChoice({ own: true, on: true, from: { kind: "project" } });
    expect(shown.selected).toBe("on");
    expect(shown.hasOn).toBe(true);
  });
});
