import {
  askToFinish,
  editAnyway,
  readDraft,
  readEditLock,
  releaseEditLock,
  sendHeartbeat,
  startHeartbeats,
  takeEditLock,
  throwAwayDraft,
  newEditorId,
  type DraftTarget,
  type EditWhat,
} from "@bugaboo/api-client";
import { useEffect, useState, useSyncExternalStore } from "react";

import { api } from "../api.ts";
import { signedInPersonId } from "../session.ts";
import { forgetBrowserDraft, readBrowserDraft, writeBrowserDraft } from "./browser-drafts.ts";
import { LongTextEditor } from "./long-text.ts";
import type { Editing, LongTextDeps } from "./long-text-model.ts";

/** A long text of a record: which one, what it is now, and how to save and read it. */
export interface LongTextOptions {
  /** The kind of record, like `issue`. */
  readonly kind: string;
  /** The internal ID of the record. */
  readonly id: string;
  /** Which long text of the record, like `description`. */
  readonly part: string;
  /** The text as saved now. */
  readonly saved: string;
  /** The version of the record now. */
  readonly version: number;
  /**
   * Saves the text under the lock; rejects with the error of the server. The function must stay
   * the same while the text is on screen (the page that shows one issue makes it once).
   */
  readonly save: LongTextDeps["save"];
  /** Reads the record again so `saved` and `version` are the ones now. Stays the same too. */
  readonly refresh: LongTextDeps["refresh"];
}

/** What names a long text and how to save and read it: the part of the options that does not change while the text is on screen. */
type Which = Omit<LongTextOptions, "saved" | "version">;

/** The calls of one long text, with the real client behind them. */
function depsOf(which: Which): LongTextDeps {
  const { kind, id, part } = which;
  const what = (): EditWhat => ({ record: { kind, id }, part });
  const target = (): DraftTarget => ({ kind, id, part });
  const client = api.client;
  return {
    take: (editor) => takeEditLock(client, what(), editor),
    peek: () => readEditLock(client, what()),
    editAnyway: (editor) => editAnyway(client, what(), editor),
    askToFinish: () => askToFinish(client, what()),
    heartbeat: (token, draft) => sendHeartbeat(client, what(), token, draft),
    release: (token) => releaseEditLock(client, what(), token),
    save: (input) => which.save(input),
    refresh: () => which.refresh(),
    serverDraft: async () => {
      const found = await readDraft(client, target());
      return found === null
        ? null
        : { text: found.text, savedAt: found.saved_at, baseVersion: found.base_version ?? null };
    },
    throwAwayServerDraft: () => throwAwayDraft(client, target()),
    // A draft is kept by person, who is asked when the draft is read or written (nobody, before
    // the app knows who is signed in: `browser-drafts.ts` keeps nothing then).
    browserDraft: {
      read: () => readBrowserDraft(signedInPersonId(), target()),
      write: (text, baseVersion) => {
        writeBrowserDraft(signedInPersonId(), target(), { text, baseVersion });
      },
      forget: () => {
        forgetBrowserDraft(signedInPersonId(), target());
      },
    },
    startHeartbeats,
    newEditorId,
  };
}

/** What a screen gets of a long text being edited: the state, and the things the person can do. */
export interface LongText extends Editing {
  readonly open: () => void;
  readonly type: (text: string) => void;
  readonly save: () => void;
  readonly cancel: () => void;
  readonly continueDraft: () => void;
  readonly dropDraft: () => void;
  readonly askToFinish: () => void;
  readonly editAnyway: () => void;
  /** *Compare and save*: the person's choice of the text, saved. */
  readonly resolve: (text: string) => void;
  /** Reads the record again, so the comparison is with the newest text. */
  readonly reread: () => void;
}

/**
 * A long text that can be edited (ISS-17): the lock on the first key, the heartbeat, drafts in the
 * browser and on the server, and the three save states. The state is in a controller, so every
 * path is tested without a browser (`long-text.ts`); this puts it on a screen.
 */
export function useLongText(options: LongTextOptions): LongText {
  const { kind, id, part, save, refresh, saved, version } = options;
  const [editor] = useState(
    () =>
      new LongTextEditor(depsOf({ kind, id, part, save, refresh }), {
        saved,
        version,
      }),
  );
  const state = useSyncExternalStore(editor.subscribe, editor.snapshot);
  useEffect(() => {
    editor.setSource({ saved, version });
  }, [editor, saved, version]);
  useEffect(() => editor.dispose, [editor]);
  return {
    ...state,
    open: editor.open,
    type: editor.type,
    save: () => {
      void editor.save();
    },
    cancel: editor.cancel,
    continueDraft: editor.continueDraft,
    dropDraft: editor.dropDraft,
    askToFinish: () => {
      void editor.askToFinish();
    },
    editAnyway: () => {
      void editor.editAnyway();
    },
    resolve: (text) => {
      void editor.resolve(text);
    },
    reread: () => {
      void editor.reread();
    },
  };
}
