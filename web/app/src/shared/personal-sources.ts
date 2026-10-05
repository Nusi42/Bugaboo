import {
  currentSource,
  parseAppearance,
  setAppearanceSource,
  type Appearance,
  type AppearanceSource,
} from "@bugaboo/ui";

import { settingKeys } from "@bugaboo/api-client";

import { changeMySettings } from "./change-my-settings.ts";
import {
  appearanceForShell,
  appearanceOf,
  changesForAppearance,
  currentMySettings,
  regionOf,
  shortcutKeysOf,
  type MySettings,
} from "./my-settings.ts";
import { queryClient } from "./query-client.ts";
import { deviceRegion, setRegion } from "./region.ts";
import {
  currentKeysSource,
  setShortcutKeysSource,
  type ShortcutKeysSource,
} from "./shortcuts-source.ts";

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

/** Forgets what this browser kept of the person's choices: the copy for the first frame. */
export function forgetPersonalChoices(): void {
  try {
    localStorage.removeItem(FIRST_FRAME_KEY);
  } catch {
    // Nothing was kept that could be removed.
  }
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
      return copy === null ? device.read() : ofCopy(copy);
    },
    write: (next) => {
      const settings = currentMySettings();
      if (settings === undefined) {
        device.write(next);
        return;
      }
      void changeMySettings(changesForAppearance(appearanceForShell(appearanceOf(settings)), next));
    },
    subscribe: (listener) => {
      const stopCache = onMySettingsChange(listener);
      const stopDevice = device.subscribe(listener);
      return () => {
        stopCache();
        stopDevice();
      };
    },
  };
}

/** The keys the person chose: from the server when signed in, else this device's (nobody's). */
function personalKeys(device: ShortcutKeysSource): ShortcutKeysSource {
  const ofSettings = remember((settings: MySettings) => shortcutKeysOf(settings));
  return {
    read: () => {
      const settings = currentMySettings();
      return settings === undefined ? device.read() : ofSettings(settings);
    },
    write: (next) => {
      if (currentMySettings() === undefined) {
        device.write(next);
        return;
      }
      const empty = Object.keys(next).length === 0;
      void changeMySettings([
        { key: settingKeys.shortcutsKeys, value: empty ? null : JSON.stringify(next) },
      ]);
    },
    subscribe: (listener) => {
      const stopCache = onMySettingsChange(listener);
      const stopDevice = device.subscribe(listener);
      return () => {
        stopCache();
        stopDevice();
      };
    },
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
    writeFirstFrame(appearanceForShell(appearanceOf(settings)));
  });
}
