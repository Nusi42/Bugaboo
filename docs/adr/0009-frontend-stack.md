# ADR-0009: Browser app: React 19, Vite, TanStack, React Aria

Status: **Accepted** · 2026-10-04

## Context

The UI must be slick, accessible (WCAG 2.2 AA), fast on phones, and buildable by agents with clear patterns.

## Decision

- **React 19** with function components and hooks; **Vite** for building and splitting.
- **TanStack Router** for type-safe routes and typed URL search parameters (searches and presentations live in the URL).
- **TanStack Query** as the only store for server data; long lists are drawn with the `Virtualizer` of React Aria Components (amendment M5).
- **React Aria Components** as the base of every interactive component (focus, keyboard, screen readers, touch).
- **Tiptap** (ProseMirror) for the document editor; **d3-force** for the relation map layout; timeline, calendar and charts are our own components on shared primitives.

## Consequences

- No global client state library: URL + query cache + local component state cover the needs.
- Heavy features are lazy-loaded to meet the 250 KB initial budget, and so are their texts: each feature brings its own part of the message catalog (ADR-0028).

## Alternatives considered

- **Next.js / server rendering:** adds a Node server to install and complicates the CSP. Rejected.
- **Headless UI kits without accessibility depth:** more work to reach WCAG. Rejected.

## Amendment M5: one way to draw long lists

Decision M5-Q11: long lists draw only the rows that can be seen with the `Virtualizer` of React Aria Components, which the people list already uses (M2-F84), not with TanStack Virtual: one library for every long list, and the keyboard and screen-reader behaviour of React Aria come with it. `@tanstack/react-virtual` is not added.

## Enforced by

ARCH-WEB-FEATURES, PERF-BUNDLE, TEST-A11Y, LINT-TS (see `tools/checks/catalog.toml`).
