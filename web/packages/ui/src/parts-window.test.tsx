import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { windowState, type WindowState } from "./props.ts";
import { WindowDock } from "./WindowDock.tsx";
import { WindowFrame, type WindowLabels, type WindowResize } from "./WindowFrame.tsx";
import { WindowLayout } from "./WindowLayout.tsx";
import { windowEdges } from "./window-size.ts";

const nothing = (): void => undefined;

/** Words of the tests, not text for people. */
const labels = {
  minimize: "Minimize",
  maximize: "Maximize",
  restoreDown: "Restore down",
  close: "Close",
  back: "Back",
  resizeKeys: "Press Ctrl and Alt with the arrow keys to change the size.",
  draftKept: "Draft kept",
  sizeMenu: "Window size",
  steps: { wider: "Wider", narrower: "Narrower", taller: "Taller", shorter: "Shorter" },
} as const satisfies WindowLabels;
const text = { title: "New issue", body: "Fields", footer: "Create" } as const;
const resize: WindowResize = { handlers: () => ({}), onKeyDown: nothing, step: nothing };

/** A window in one of its states, as markup. */
function frame(state: WindowState, footer?: ReactNode): string {
  return renderToStaticMarkup(
    <WindowFrame
      title={text.title}
      labels={labels}
      state={state}
      footer={footer}
      onMinimize={nothing}
      onToggleMaximize={nothing}
      onClose={nothing}
      resize={resize}
    >
      {text.body}
    </WindowFrame>,
  );
}

/** The names of the buttons of the title bar, in the order they stand. */
function buttonNames(markup: string): string[] {
  return markup
    .split("<button")
    .slice(1)
    .map(
      (button) =>
        button.split('aria-label="')[1]?.split('"')[0] ??
        (button.split(">").slice(1).join(">").split("</button>")[0] ?? "").replaceAll(
          /<[^>]*>/gu,
          "",
        ),
    );
}

describe("ux_6_3 a window has its title bar, its form and its footer", () => {
  it("has Minimize, Maximize and Close at the top right, in the order of Windows, after the size menu", () => {
    const markup = frame(windowState.normal);
    expect(buttonNames(markup)).toEqual([
      labels.sizeMenu,
      labels.minimize,
      labels.maximize,
      labels.close,
    ]);
    expect(markup).toContain('data-part="window.title-bar"');
    expect(markup).toContain(`>${text.title}</h2>`);
    expect(markup).toContain('data-variant="close"');
  });

  it("offers Restore down instead of Maximize once it is maximized, and no place to drag or size menu", () => {
    const markup = frame(windowState.maximized);
    expect(buttonNames(markup)).toEqual([labels.minimize, labels.restoreDown, labels.close]);
    expect(markup).not.toContain('data-part="window.handle"');
  });

  it("is a full screen with Back on a phone: no resize, no minimize, no maximize", () => {
    const markup = frame(windowState.full);
    expect(buttonNames(markup)).toEqual([labels.back]);
    expect(markup).not.toContain('data-part="window.handle"');
  });

  it("keeps the title bar and the footer outside the part that scrolls", () => {
    const markup = frame(windowState.normal, text.footer);
    const body = markup.indexOf('data-part="window.body"');
    expect(markup.indexOf('data-part="window.title-bar"')).toBeLessThan(body);
    expect(markup.indexOf('data-part="window.footer"')).toBeGreaterThan(body);
    expect(markup).not.toContain("style=");
  });
});

describe("ux_04 a window can be resized at every edge and corner, also with the keyboard", () => {
  it("has a place to drag at each edge and each corner of the theme file", () => {
    const markup = frame(windowState.normal);
    for (const edge of windowEdges) {
      expect(markup).toContain(`data-part="window.handle" data-variant="${edge}"`);
    }
  });

  it("says how to resize with the keys, to the window and to screen readers", () => {
    const markup = frame(windowState.normal);
    expect(markup).toContain(labels.resizeKeys);
    expect(markup).toContain("aria-describedby");
  });
});

describe("ux_6_3 a minimized window is a bar at the bottom", () => {
  it("shows the title and what it keeps, and opens the window again when it is pressed", () => {
    const markup = renderToStaticMarkup(
      <WindowDock title={text.title} note={labels.draftKept} onRestore={nothing} />,
    );
    expect(markup).toContain('data-part="window-dock"');
    expect(markup).toContain(text.title);
    expect(markup).toContain(labels.draftKept);
  });
});

describe("ux_6_3 the form follows the window's own width", () => {
  it("has a text and the fields, whose columns the container queries of the theme file set", () => {
    const markup = renderToStaticMarkup(
      <WindowLayout fields={text.footer}>{text.body}</WindowLayout>,
    );
    expect(markup).toContain('data-part="window-layout"');
    expect(markup).toContain('data-part="window-layout.text"');
    expect(markup).toContain('data-part="window-layout.fields"');
  });
});
