import type { Schema } from "@bugaboo/api-client";
import { useNavigate } from "@tanstack/react-router";

import { api } from "../../shared/api.ts";
import { fieldAddress } from "../../shared/paths.ts";
import { refreshFields } from "../../shared/query-keys.ts";
import type { Field } from "./fields.ts";

/** The routes that change an own field or one of its choices: each reads every field again when it succeeds. */
export function useChangeField() {
  return api.query.useMutation("patch", "/api/v1/fields/{field}", { onSuccess: refreshFields });
}

/** Makes a field, then opens its page. */
export function useMakeField() {
  const navigate = useNavigate();
  return api.query.useMutation("post", "/api/v1/fields", {
    onSuccess: (field: Schema<"FieldBody">) => {
      refreshFields();
      void navigate({ to: fieldAddress(field.id) });
    },
  });
}

/** Adds a choice. */
export function useAddChoice() {
  return api.query.useMutation("post", "/api/v1/fields/{field}/choices", {
    onSuccess: refreshFields,
  });
}

/** Changes a choice. */
export function useChangeChoice() {
  return api.query.useMutation("patch", "/api/v1/fields/{field}/choices/{choice}", {
    onSuccess: refreshFields,
  });
}

/** Puts the choices in a new order. */
export function useReorderChoices() {
  return api.query.useMutation("put", "/api/v1/fields/{field}/choices/order", {
    onSuccess: refreshFields,
  });
}

/** Combines a choice into another. */
export function useCombineChoices() {
  return api.query.useMutation("post", "/api/v1/fields/{field}/choices/{choice}/combine", {
    onSuccess: refreshFields,
  });
}

/** The request that changes some members of a field, on the version the person looked at. */
export function fieldChange(field: Field, more: Readonly<Record<string, unknown>>) {
  return {
    params: { path: { field: field.id } },
    body: { version: field.version, ...more },
  };
}
