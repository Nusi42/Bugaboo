import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { backupRows, chosenName, createState, makerOf } from "./backups.ts";

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
    const body: Backups = { files: [file("b.backup"), file("a.backup")], runs: [], asked: null };
    expect(backupRows(body).map((row) => row.name)).toEqual(["b.backup", "a.backup"]);
  });
});

describe("the person of a row", () => {
  it("is the person the record of the backup names", () => {
    const person = { shown: "name", id: "u1", name: "Ada" } as const;
    const body: Backups = {
      files: [file("a.backup")],
      runs: [{ ...run("a.backup", "person"), person }],
      asked: null,
    };
    expect(backupRows(body)[0]?.person).toEqual(person);
    expect(backupRows({ files: [file("a.backup")], runs: [], asked: null })[0]?.person).toBeNull();
  });
});

describe("the backup a restore command is shown for", () => {
  const rows = backupRows({ files: [file("b.backup"), file("a.backup")], runs: [], asked: null });

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

describe("the words beside Create a backup now", () => {
  const idle = { isPending: false, isError: false, isSuccess: false };
  const running = { status: "running", asked_at: "2026-11-18T03:15:00Z" } as const;
  const failed = { ...running, status: "failed" } as const;

  it("say creating while the ask is sent and while the server makes the backup", () => {
    expect(createState(null, { ...idle, isPending: true })).toBe("saving");
    expect(createState(running, { ...idle, isSuccess: true })).toBe("saving");
    // Also after a reload, when this page did not ask.
    expect(createState(running, idle)).toBe("saving");
  });

  it("say created once the backup this page asked for is there", () => {
    expect(createState(null, { ...idle, isSuccess: true })).toBe("saved");
    expect(createState(null, idle)).toBe("idle");
  });

  it("say not created when the ask is refused or the backup failed", () => {
    expect(createState(null, { ...idle, isError: true })).toBe("not-saved");
    expect(createState(failed, idle)).toBe("not-saved");
    expect(createState(failed, { ...idle, isSuccess: true })).toBe("not-saved");
  });

  it("name the backup, as they stand alone beside the button", () => {
    for (const words of [
      t("backup-page.creating"),
      t("backup-page.created"),
      t("backup-page.not-created"),
    ]) {
      expect(words.toLowerCase()).toContain("backup");
    }
  });
});
