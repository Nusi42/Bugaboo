import { describe, expect, it } from "vitest";

import { joinedModules, type ModuleLinks } from "./joined-chunk.ts";

/** A small graph: the entry, the joined file and what the features it loads need. */
const graph: Readonly<Record<string, ModuleLinks>> = {
  "/web/app/src/main.tsx": {
    importedIds: ["/web/app/src/routes/joined.ts", "/react"],
    dynamicallyImportedIds: [],
    isEntry: true,
  },
  "/web/app/src/routes/joined.ts": {
    importedIds: [],
    dynamicallyImportedIds: [
      "/web/app/src/features/pins/index.ts",
      "/web/app/src/features/dashboards/index.ts",
    ],
    isEntry: false,
  },
  "/react": { importedIds: [], dynamicallyImportedIds: [], isEntry: false },
  "/web/app/src/features/pins/index.ts": {
    importedIds: ["/react", "/web/packages/ui/MoreButton.tsx"],
    dynamicallyImportedIds: [],
    isEntry: false,
  },
  "/web/app/src/features/dashboards/index.ts": {
    importedIds: ["/web/packages/messages/parts/dashboards.ts"],
    dynamicallyImportedIds: ["/web/app/src/features/dashboards/HomeBoxes.tsx"],
    isEntry: false,
  },
  "/web/packages/ui/MoreButton.tsx": {
    importedIds: [],
    dynamicallyImportedIds: [],
    isEntry: false,
  },
  "/web/packages/messages/parts/dashboards.ts": {
    importedIds: [],
    dynamicallyImportedIds: [],
    isEntry: false,
  },
  "/web/app/src/features/dashboards/HomeBoxes.tsx": {
    importedIds: [],
    dynamicallyImportedIds: [],
    isEntry: false,
  },
};

const linksOf = (id: string): ModuleLinks | null => graph[id] ?? null;

describe("the file of the features every signed-in first page waits for", () => {
  it("holds the app code the joined features need statically, not the components, the first file or what loads later", () => {
    expect([...joinedModules(Object.keys(graph), linksOf)].toSorted()).toEqual([
      "/web/app/src/features/dashboards/index.ts",
      "/web/app/src/features/pins/index.ts",
      "/web/packages/messages/parts/dashboards.ts",
    ]);
  });

  it("holds nothing when there is no joined file", () => {
    expect(joinedModules(["/react"], linksOf).size).toBe(0);
  });
});
