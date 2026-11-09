import { describe, expect, it } from "vitest";

import { areaAt } from "./places.ts";

/** Addresses of the tests, not text for people. */
const address = {
  home: "/",
  issue: "/issues/WEB-1",
  projectIssues: "/p/WEB/issues",
  timesheet: "/time",
  workingHoursOfOthers: "/time/people",
  myAgents: "/me/agents",
  oneAgent: "/me/agents/a1",
  mySettings: "/me",
  organizationSettings: "/settings",
  aTopic: "/settings/people",
  aTopicPage: "/settings/people/p1",
  projectSettings: "/p/WEB/settings/roles",
  unknown: "/nowhere",
} as const;

describe("the area of an address", () => {
  it("is issues for the start page, issues, projects and what is not known", () => {
    for (const path of [address.home, address.issue, address.projectIssues, address.unknown]) {
      expect(areaAt(path)).toBe("issues");
    }
  });

  it("is time under the timesheet", () => {
    expect(areaAt(address.timesheet)).toBe("time");
    expect(areaAt(address.workingHoursOfOthers)).toBe("time");
  });

  it("keeps My agents apart from My settings, which share the start of their address", () => {
    expect(areaAt(address.myAgents)).toBe("agents");
    expect(areaAt(address.oneAgent)).toBe("agents");
    expect(areaAt(address.mySettings)).toBe("settings");
  });

  it("is settings for the settings of the organization and of a project", () => {
    for (const path of [
      address.organizationSettings,
      address.aTopic,
      address.aTopicPage,
      address.projectSettings,
    ]) {
      expect(areaAt(path)).toBe("settings");
    }
  });
});
