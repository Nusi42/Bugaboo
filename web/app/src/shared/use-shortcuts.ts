import { useEffect, useLayoutEffect, useRef, useSyncExternalStore } from "react";

import { createEngine } from "./shortcuts-engine.ts";
import { currentKeysSource } from "./shortcuts-source.ts";
import type { ShortcutHandlers, ShortcutId, ShortcutOverrides } from "./shortcuts.ts";

/** What the screens on show do for each shortcut. The one registered last has the say. */
const registrations: { readonly current: () => ShortcutHandlers }[] = [];

/** The handlers of every screen on show, the latest winning for a shortcut they share. */
function mergedHandlers(): ShortcutHandlers {
  const merged: Partial<Record<ShortcutId, () => void>> = {};
  for (const registration of registrations) {
    Object.assign(merged, registration.current());
  }
  return merged;
}

/**
 * Says what the shortcuts do on this screen (UX-05). A shortcut that no mounted screen handles does
 * nothing. Every shortcut also has a visible button (principle 9), so a handler here is never the
 * only way to do the thing.
 */
export function useShortcuts(handlers: ShortcutHandlers): void {
  const latest = useRef(handlers);
  useLayoutEffect(() => {
    latest.current = handlers;
  });
  useEffect(() => {
    const registration = { current: () => latest.current };
    registrations.push(registration);
    return () => {
      registrations.splice(registrations.indexOf(registration), 1);
    };
  }, []);
}

/**
 * Listens to the keyboard for the whole page. Mount it once, in the frame of the signed-in app.
 * A key press that is a shortcut does not go on to the page (Ctrl+K must not open the search of
 * the browser).
 */
export function useShortcutHost(): void {
  useEffect(() => {
    const engine = createEngine({
      overrides: () => currentKeysSource().read(),
      handlers: mergedHandlers,
      now: () => Date.now(),
    });
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.defaultPrevented) {
        return;
      }
      const target = event.target instanceof HTMLElement ? event.target : null;
      const pressed = engine({
        key: event.key,
        ctrlKey: event.ctrlKey,
        metaKey: event.metaKey,
        altKey: event.altKey,
        shiftKey: event.shiftKey,
        isComposing: event.isComposing,
        target,
      });
      if (pressed) {
        event.preventDefault();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);
}

/** The keys the person chose instead of the defaults; it follows changes. */
export function useShortcutOverrides(): ShortcutOverrides {
  return useSyncExternalStore(
    (listener) => currentKeysSource().subscribe(listener),
    () => currentKeysSource().read(),
  );
}

/** Uses these keys for a shortcut, or the defaults again when `keys` is `null`. */
export function setShortcutKeys(id: ShortcutId, keys: readonly string[] | null): void {
  const source = currentKeysSource();
  const next: Partial<Record<ShortcutId, readonly string[]>> = { ...source.read() };
  if (keys === null) {
    delete next[id];
  } else {
    next[id] = keys;
  }
  source.write(next);
}
