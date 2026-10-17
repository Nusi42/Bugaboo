import { layoutPlaces } from "./layout-places.ts";
import type { Place } from "./place.ts";
import { workflowPlaces } from "./workflow-places.ts";

/** The screens of *Organization settings › Work* that sit next to each other: layouts and workflows. */
export const workPlaces: readonly Place[] = [...layoutPlaces, ...workflowPlaces];
