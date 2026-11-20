import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { downloadKinds, supportOf } from "./about.ts";

describe("the support period", () => {
  it("is only the newest release for a preview", () => {
    expect(supportOf({ released: "2026-11-01", fixes_until: null })).toEqual({
      kind: "newest-only",
    });
  });

  it("is the last day of fixes from 1.0", () => {
    expect(supportOf({ released: "2027-01-01", fixes_until: "2028-01-01" })).toEqual({
      kind: "until",
      day: "2028-01-01",
    });
  });
});

describe("the downloads", () => {
  type Files = Schema<"AboutBody">["files"];
  const files: Files = [
    { kind: "notices", name: "n.txt", address: "/api/v1/about/files/notices" },
    { kind: "spdx", name: "s.json", address: "/api/v1/about/files/spdx" },
    { kind: "cyclonedx", name: "c.json", address: "/api/v1/about/files/cyclonedx" },
    { kind: "vex", name: "v.json", address: "/api/v1/about/files/vex" },
  ];

  it("are the parts list in both formats and the notices, in a fixed order", () => {
    expect(downloadKinds(files).map((each) => each.kind)).toEqual(["cyclonedx", "spdx", "notices"]);
  });

  it("leave out what the build does not carry", () => {
    expect(downloadKinds(files.slice(0, 1)).map((each) => each.kind)).toEqual(["notices"]);
  });
});
