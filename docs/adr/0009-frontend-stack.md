# ADR-0009: Browser app: React 19, Vite, TanStack, React Aria

Status: **Accepted** · 2026-10-04

## Context

The UI must be slick, accessible (WCAG 2.2 AA), fast on phones, and buildable by agents with clear patterns.

## Decision

- **React 19** with function components and hooks; **Vite** for building and splitting.
- **TanStack Router** for type-safe routes and typed URL search parameters (searches and presentations live in the URL).
- **TanStack Query** as the only store for server data; long lists are drawn with the `Virtualizer` of React Aria Components (amendment M5).
- **React Aria Components** as the base of every interactive component (focus, keyboard, screen readers, touch).
- **Tiptap** (ProseMirror) for the document editor; **d3-force** for the relation map layout *(amended: the map has a layout of our own; see the amendment of 2026-11-24)*; timeline, calendar and charts are our own components on shared primitives.

## Consequences

- No global client state library: URL + query cache + local component state cover the needs.
- Heavy features are lazy-loaded to meet the 250 KB initial budget, and so are their texts: each feature brings its own part of the message catalog (ADR-0028).

## Alternatives considered

- **Next.js / server rendering:** adds a Node server to install and complicates the CSP. Rejected.
- **Headless UI kits without accessibility depth:** more work to reach WCAG. Rejected.

## Amendment M5: one way to draw long lists

Decision M5-Q11: long lists draw only the rows that can be seen with the `Virtualizer` of React Aria Components, which the people list already uses (M2-F84), not with TanStack Virtual: one library for every long list, and the keyboard and screen-reader behaviour of React Aria come with it. `@tanstack/react-virtual` is not added.

## Amendment (2026-11-24, the relation map's own layout)

The maintainer took the recommendation of M10-Q10 of [the M10 plan](../milestones/M10/README.md). A force layout keeps moving while it settles, which breaks *Nothing moves under the cursor* (UX design §6.4), and gives a different picture each time, which a visual test cannot compare. Decided:

1. **d3-force is not added.** The relation map is laid out by a component of our own, on the shared primitives, like the timeline, the calendar and the charts.
2. **Columns by steps away.** The starting issue stands in the middle; the issues it waits for and its parents stand in columns to the left, the ones that wait for it and its children to the right, one column per step away, and the kinds without a direction by their first step. Inside a column the issues have a fixed order, so the same answer always gives the same picture.
3. **Nothing moves by itself.** The picture is drawn once from the answer; it changes only when the person changes the steps, the *Kinds shown* or the issue in the middle (*Put in the middle*).

Enforced by (additional): DEP-UNUSED and DEP-REGISTER (no layout library is added), review (UX design §6.4: the map is measured for movement like every new screen).

## Enforced by

ARCH-WEB-FEATURES, PERF-BUNDLE, TEST-A11Y, LINT-TS (see `tools/checks/catalog.toml`).
