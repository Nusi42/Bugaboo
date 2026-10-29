import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

describe("ux_5 choosing a role to look at", () => {
  it("is said in one way where the levels of a role and what a role sees are shown", () => {
    expect(t("people-visibility.preview-role")).toBe(t("access.role-chooser"));
    expect(t("access.role-chooser")).toBe(t("assignments.choose-role"));
  });
});
