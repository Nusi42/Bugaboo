import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { backupRows, chosenName, makerOf } from "./backups.ts";

type File = Schema<"BackupFileBody">;
type Run = Schema<"BackupRunBody">;
type Backups = Schema<"BackupsBody">;

function file(name: string, extra: Partial<File> = {}): File {
  return {
    name,
    moment: "2026-11-18T03:15:00Z",
    version: "0.6.0",
    before_restore: false,
    size: 2048,
    ...extra,
  };
}

function run(name: string, maker: string): Run {
  return {
    event: "made",
    name,
    size: 2048,
    checksum: "0".repeat(64),
    maker,
    at: "2026-11-18T03:15:00Z",
  };
}

describe("who made a backup", () => {
  it("is read from the record of the backup with the same name", () => {
    const runs = [run("a.backup", "schedule"), run("b.backup", "person")];
    expect(makerOf(file("a.backup"), runs)).toBe("schedule");
    expect(makerOf(file("b.backup"), runs)).toBe("person");
    expect(makerOf(file("c.backup"), runs)).toBe("unknown");
  });

  it("is the restore for a backup made before one, whatever the records say", () => {
    expect(makerOf(file("a.backup", { before_restore: true }), [run("a.backup", "schedule")])).toBe(
      "before-restore",
    );
  });

  it("is unknown for a maker this version does not know", () => {
    expect(makerOf(file("a.backup"), [run("a.backup", "a-new-maker")])).toBe("unknown");
  });

  it("counts only the records of backups that were made, not of restores", () => {
    const restored: Run = { ...run("a.backup", "command-line"), event: "restored" };
    expect(makerOf(file("a.backup"), [restored])).toBe("unknown");
  });
});

describe("the rows of the list", () => {
  it("keep the order of the server, which is newest first", () => {
    const body: Backups = { files: [file("b.backup"), file("a.backup")], runs: [] };
    expect(backupRows(body).map((row) => row.name)).toEqual(["b.backup", "a.backup"]);
  });
});

describe("the person of a row", () => {
  it("is the person the record of the backup names", () => {
    const person = { shown: "name", id: "u1", name: "Ada" } as const;
    const body: Backups = {
      files: [file("a.backup")],
      runs: [{ ...run("a.backup", "person"), person }],
    };
    expect(backupRows(body)[0]?.person).toEqual(person);
    expect(backupRows({ files: [file("a.backup")], runs: [] })[0]?.person).toBeNull();
  });
});

describe("the backup a restore command is shown for", () => {
  const rows = backupRows({ files: [file("b.backup"), file("a.backup")], runs: [] });

  it("is the chosen one while it is in the list", () => {
    expect(chosenName(rows, "a.backup")).toBe("a.backup");
  });

  it("is the newest when nothing is chosen or the chosen one is gone", () => {
    expect(chosenName(rows, null)).toBe("b.backup");
    expect(chosenName(rows, "gone.backup")).toBe("b.backup");
  });

  it("is nothing when there is no backup", () => {
    expect(chosenName([], null)).toBeUndefined();
  });
});
