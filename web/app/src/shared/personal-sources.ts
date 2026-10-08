import {
  currentSource,
  defaultAppearance,
  forgetStoredAppearance,
  parseAppearance,
  setAppearanceSource,
  type Appearance,
  type AppearanceSource,
} from "@bugaboo/ui";

import { settingKeys } from "@bugaboo/api-client";

import { changeMySettings, reportNotSaved } from "./change-my-settings.ts";
import { codeOf } from "./errors.ts";
import { appearanceForShell, appearanceOf, changesForAppearance } from "./appearance-setting.ts";
import { currentMySettings, mySettingsKey, type MySettings } from "./my-settings.ts";
import { queryClient } from "./query-client.ts";
import { deviceRegion, regionOf, setRegion } from "./region.ts";
import {
  currentKeysSource,
  forgetStoredKeys,
  setShortcutKeysSource,
  shortcutKeysOf,
  type ShortcutKeysSource,
} from "./shortcuts-source.ts";
import { type ShortcutOverrides } from "./shortcuts.ts";

/**
 * The copy of the look that this browser keeps only for the first frame (M0-F05): before the
 * server has answered, the page is drawn with it, so a person who chose a theme does not see
 * another for a moment. It goes with the sign-out, so the next person at this device finds
 * nothing of the last person's choices (M2-F13).
 */
const FIRST_FRAME_KEY = "bugaboo.first-frame";

function readFirstFrame(): string | null {
  try {
    return localStorage.getItem(FIRST_FRAME_KEY);
  } catch {
    return null;
  }
}

function writeFirstFrame(look: Appearance): void {
  try {
    localStorage.setItem(FIRST_FRAME_KEY, JSON.stringify(look));
  } catch {
    // Without it the first frame is the device's, which is all it was before.
  }
}

/**
 * Choices the person made while the server could not be reached, kept in this page only: never in
 * the browser's storage, where the next person at this device could find them.
 */
const unsent: { look: Appearance | null; keys: ShortcutOverrides | null } = {
  look: null,
  keys: null,
};
const unsentWatchers = new Set<() => void>();

function tellUnsentWatchers(): void {
  for (const watcher of unsentWatchers) {
    watcher();
  }
}

/** Forgets what this browser kept of the person's choices: the copy for the first frame. */
export function forgetPersonalChoices(): void {
  try {
    localStorage.removeItem(FIRST_FRAME_KEY);
  } catch {
    // Nothing was kept that could be removed.
  }
  unsent.look = null;
  unsent.keys = null;
}

/**
 * Removes the choices this device kept before the person's choices lived on the server (the look
 * and the keys of the pages before sign-in): once somebody is signed in, nothing of them is left
 * in the storage of the browser to be found by the next person.
 */
export function forgetDeviceChoices(): void {
  forgetStoredAppearance();
  forgetStoredKeys();
}

/**
 * Whether somebody is signed in but the app has no settings of theirs: the server failed to give
 * them, or does not answer yet (the screen that asks for a second step comes before any settings
 * can be read). The copy for the first frame says who is signed in: it is only there from the
 * moment their settings were read until they sign out. Their changes then stay in this page, and
 * the page says *Not saved yet*.
 */
function signedInWithoutSettings(): boolean {
  if (currentMySettings() !== undefined) {
    return false;
  }
  if (readFirstFrame() !== null) {
    return true;
  }
  const state = queryClient.getQueryState(mySettingsKey());
  return state?.status === "error" && codeOf(state.error) !== "auth.not_signed_in";
}

/**
 * Calls `listener` when the person's settings in the cache are another answer than before (they
 * came, changed or went with the sign-out). Returns the way to stop.
 */
function onMySettingsChange(listener: () => void): () => void {
  let seen = currentMySettings();
  return queryClient.getQueryCache().subscribe(() => {
    const now = currentMySettings();
    if (now !== seen) {
      seen = now;
      listener();
    }
  });
}

/** A value that is made again only when what it is made from is another one. */
function remember<Input extends object | string, Output>(
  make: (input: Input) => Output,
): (input: Input) => Output {
  let last: { readonly input: Input; readonly output: Output } | null = null;
  return (input) => {
    if (last?.input !== input) {
      last = { input, output: make(input) };
    }
    return last.output;
  };
}

/**
 * The look: the person's settings from the server when they are signed in and the app has them,
 * else the copy kept for the first frame, else what this device chose (the choice at the bottom
 * of the pages before sign-in, which is the device's and nobody's account).
 */
function personalAppearance(device: AppearanceSource): AppearanceSource {
  const ofSettings = remember((settings: MySettings) => appearanceForShell(appearanceOf(settings)));
  const ofCopy = remember((text: string) => parseAppearance(text));
  return {
    read: () => {
      const settings = currentMySettings();
      if (settings !== undefined) {
        return ofSettings(settings);
      }
      const copy = readFirstFrame();
      if (copy !== null) {
        return unsent.look ?? ofCopy(copy);
      }
      return signedInWithoutSettings() ? (unsent.look ?? defaultAppearance) : device.read();
    },
    write: (next) => {
      const settings = currentMySettings();
      if (settings === undefined) {
        if (signedInWithoutSettings()) {
          unsent.look = next;
          reportNotSaved();
          tellUnsentWatchers();
          return;
        }
        device.write(next);
        return;
      }
      void changeMySettings(changesForAppearance(appearanceForShell(appearanceOf(settings)), next));
    },
    subscribe: (listener) => subscribeToAll(device, listener),
  };
}

/** Calls `listener` when the settings in the cache, the device's choice or an unsent one changes. */
function subscribeToAll(
  device: { readonly subscribe: (listener: () => void) => () => void },
  listener: () => void,
): () => void {
  unsentWatchers.add(listener);
  const stopCache = onMySettingsChange(listener);
  const stopDevice = device.subscribe(listener);
  return () => {
    unsentWatchers.delete(listener);
    stopCache();
    stopDevice();
  };
}

/** The keys the person chose: from the server when signed in, else this device's (nobody's). */
function personalKeys(device: ShortcutKeysSource): ShortcutKeysSource {
  const ofSettings = remember((settings: MySettings) => shortcutKeysOf(settings));
  return {
    read: () => {
      const settings = currentMySettings();
      if (settings !== undefined) {
        return ofSettings(settings);
      }
      return signedInWithoutSettings() ? (unsent.keys ?? {}) : device.read();
    },
    write: (next) => {
      if (currentMySettings() === undefined) {
        if (signedInWithoutSettings()) {
          unsent.keys = next;
          reportNotSaved();
          tellUnsentWatchers();
          return;
        }
        device.write(next);
        return;
      }
      const empty = Object.keys(next).length === 0;
      void changeMySettings([
        { key: settingKeys.shortcutsKeys, value: empty ? null : JSON.stringify(next) },
      ]);
    },
    subscribe: (listener) => subscribeToAll(device, listener),
  };
}

/**
 * Makes the app read and write the person's own choices (look, shortcut keys, region) through the
 * server. Call it once at the start, before the first page is drawn. Where nobody is signed in,
 * the choices stay the device's, as the pages before sign-in have them.
 */
export function installPersonalSources(): void {
  setAppearanceSource(personalAppearance(currentSource()));
  setShortcutKeysSource(personalKeys(currentKeysSource()));
  onMySettingsChange(() => {
    const settings = currentMySettings();
    if (settings === undefined) {
      setRegion(deviceRegion);
      return;
    }
    setRegion(regionOf(settings));
    forgetDeviceChoices();
    writeFirstFrame(appearanceForShell(appearanceOf(settings)));
  });
}
