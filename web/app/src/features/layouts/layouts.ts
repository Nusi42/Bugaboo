import { dataOf, layoutKindWords, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { refreshLayouts } from "../../shared/query-keys.ts";

/** What a layout is for: the issue page or the new-issue form. */
export type FormKind = (typeof layoutKindWords)[number];
/** The layout of a place as the editor shows it. */
export type LayoutEditor = Schema<"LayoutEditorBody">;
/** A section of the form as one person sees it. */
export type FormSection = Schema<"FormSectionBody">;

/** The form a layout is for when nothing says. */
export const startingKind: FormKind = "new-issue";

/** What a form is called: the name of its kind. */
export function kindName(kind: FormKind): string {
  const names = {
    "new-issue": t("layout-kind.new-issue"),
    "issue-page": t("layout-kind.issue-page"),
  } as const satisfies Record<FormKind, string>;
  return names[kind];
}

/** The kind a word of an address names, if it names one. */
export function kindOf(word: string | undefined): FormKind {
  return layoutKindWords.find((kind) => kind === word) ?? startingKind;
}

/** Which form to ask about: the project, the issue type of the issue, and what the form is for. */
export interface FormQuery {
  /** The project, by its short name; nothing while it is not known yet. */
  readonly project: string | undefined;
  /** The ID of the issue type; nothing while none is picked. */
  readonly issueType: string | undefined;
  readonly kind: FormKind;
}

/** The form for one person: its sections, and whether a field is on it. */
export interface FormView {
  /** The layout has been read; before that every field counts as shown, so nothing flashes away. */
  readonly ready: boolean;
  readonly sections: readonly FormSection[];
  /** Whether the layout shows the field to this person (a built-in word or the ID of an own field). */
  readonly shows: (field: string) => boolean;
}

/** Everything is shown: what a form is while its layout is on the way. */
const everything: FormView = { ready: false, sections: [], shows: () => true };

/**
 * The one hook that the issue page and the new-issue form call to learn which fields they show
 * (CFG-01): the layout of the project and issue type, applied on the server to the roles and
 * the levels of the person, so a field they may not see (or fill in) is not in it.
 */
export function useFormLayout({ project, issueType, kind }: FormQuery): FormView {
  const form = useQuery({
    ...api.query.queryOptions("get", "/api/v1/layouts/form", {
      params: {
        query: {
          project: project ?? "",
          kind,
          ...(issueType === undefined || issueType === "" ? {} : { issue_type: issueType }),
        },
      },
    }),
    select: (body: Schema<"FormLayoutBody">) => body,
    enabled: project !== undefined && project !== "",
  });
  if (form.data === undefined) {
    return everything;
  }
  const fields = new Set(form.data.sections.flatMap((section) => section.fields));
  return {
    ready: true,
    sections: form.data.sections,
    shows: (field) => fields.has(field),
  };
}

/** Which layout to edit: a project (none for the whole organization), an issue type (none for all) and a form. */
export interface EditorQuery {
  readonly project: string | undefined;
  readonly issueType: string | undefined;
  readonly kind: FormKind;
}

/** The layout of a place for somebody who may change it. */
export function useLayoutEditor({ project, issueType, kind }: EditorQuery) {
  return useQuery({
    ...api.query.queryOptions("get", "/api/v1/layouts/editor", {
      params: {
        query: {
          kind,
          ...(project === undefined ? {} : { project }),
          ...(issueType === undefined ? {} : { issue_type: issueType }),
        },
      },
    }),
    select: (body: LayoutEditor) => body,
  });
}

/** *Customize this form*: makes the layout of the place as a copy of the one that counts there. */
export function useMakeLayout({ project, issueType, kind }: EditorQuery) {
  return useMutation({
    mutationFn: async () =>
      dataOf(
        await api.client.POST("/api/v1/layouts", {
          body: {
            kind,
            ...(project === undefined ? {} : { project }),
            ...(issueType === undefined ? {} : { issue_type: issueType }),
          },
        }),
      ),
    onSuccess: refreshLayouts,
  });
}

/** Saves the sections of a layout under the edit lock the person holds. */
export function useSaveLayout() {
  return useMutation({
    mutationFn: async (input: {
      readonly layout: string;
      readonly version: number;
      readonly lockToken: string;
      readonly sections: Schema<"SaveLayoutBody">["sections"];
    }): Promise<LayoutEditor> =>
      dataOf(
        await api.client.PUT("/api/v1/layouts/{layout}", {
          params: { path: { layout: input.layout } },
          body: {
            version: input.version,
            lock_token: input.lockToken,
            sections: input.sections.map((section) => ({
              tab: section.tab,
              title: section.title,
              fields: section.fields.map((field) => ({
                field: field.field,
                roles: [...field.roles],
              })),
            })),
          },
        }),
      ),
    onSuccess: refreshLayouts,
  });
}

/** Takes the layout of a place away: the place uses the layout it inherits again. */
export function useDeleteLayout() {
  return useMutation({
    mutationFn: async (input: { readonly layout: string; readonly version: number }) =>
      dataOf(
        await api.client.DELETE("/api/v1/layouts/{layout}", {
          params: { path: { layout: input.layout }, query: { version: input.version } },
        }),
      ),
    onSuccess: refreshLayouts,
  });
}
