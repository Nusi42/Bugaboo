import { ApiError, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook } from "@bugaboo/ui";
import { useCallback, useState, type ReactNode } from "react";

import { api } from "./api.ts";
import type { PermissionKey } from "./permissions.ts";

/** A change that is about to be made, as the server describes it. */
export type OwnChange = Schema<"OwnChangeBody">;

/** The change is waiting for the answer of the person. */
interface Waiting {
  readonly lost: readonly PermissionKey[];
  readonly settle: (go: boolean) => void;
}

/** What the change would take from the person who makes it; nothing when the server cannot say. */
async function whatItTakes(change: OwnChange): Promise<readonly PermissionKey[]> {
  try {
    // The list of permissions is copied: the request wants a list that can be changed.
    const body =
      change.kind === "change-role" ? { ...change, permissions: [...change.permissions] } : change;
    const { data } = await api.client.POST("/api/v1/own-access/check-change", { body });
    return data?.loses ?? [];
  } catch (error) {
    if (!(error instanceof ApiError)) {
      throw error;
    }
    // The change itself reports its own failure, so a question that could not be asked is no reason to hold it back.
    return [];
  }
}

/** The sentence that says what would be lost. */
function lostText(lost: readonly PermissionKey[]): string {
  return lost
    .map((permission) =>
      t(permission === "manage-roles" ? "own.lost-manage-roles" : "own.lost-decide-roles"),
    )
    .join(" ");
}

/** What the page that makes changes gets: `ask` and the question to draw. */
export interface AskFirst {
  /**
   * Asks the server what the change would take from the person. Resolves `true` when it takes
   * nothing, or when the person says to go on after being told; `false` when they keep what they have.
   */
  readonly ask: (change: OwnChange) => Promise<boolean>;
  /** The question; draw it once on the page. */
  readonly dialog: ReactNode;
}

/**
 * The question before a change that would take away the person's own way to manage roles or to
 * decide who has which role (SEC-08). The engine answers whether it would, from the same rows and
 * roles it decides with, so the question comes exactly then and for every way of losing it: the
 * row is theirs or their group's, *No access* on themselves, leaving a group, changing or deleting
 * a role they hold.
 */
export function useAskFirst(): AskFirst {
  const [waiting, setWaiting] = useState<Waiting | null>(null);
  const ask = useCallback(async (change: OwnChange): Promise<boolean> => {
    const lost = await whatItTakes(change);
    if (lost.length === 0) {
      return true;
    }
    return new Promise<boolean>((settle) => {
      setWaiting({ lost, settle });
    });
  }, []);
  const close = (go: boolean): void => {
    waiting?.settle(go);
    setWaiting(null);
  };
  const dialog = (
    <AskDialog
      isOpen={waiting !== null}
      onOpenChange={(open) => {
        if (!open) {
          close(false);
        }
      }}
      title={t("own.ask-title")}
      text={t("own.ask-text", { lost: lostText(waiting?.lost ?? []) })}
      confirmLabel={t("own.ask-yes")}
      cancelLabel={t("own.ask-keep")}
      confirmLook={buttonLook.danger}
      onConfirm={() => {
        close(true);
      }}
    />
  );
  return { ask, dialog };
}
