import type { AnnotationLocation } from "./types";

export type AnnotationStatus = "draft" | "submitted" | "orphaned";

export type AnnotationPayload = Record<string, string>;

export type AnnotationDraft = {
  id: string;
  location: AnnotationLocation;
  ruleCode: string | null;
  payload: AnnotationPayload;
  status: AnnotationStatus;
  updatedAt: number;
};

export type AnnotationState = {
  items: AnnotationDraft[];
  activeId: string | null;
};

export type AnnotationAction =
  | { type: "OPEN"; location: AnnotationLocation }
  | { type: "UPDATE_RULE"; id?: string; ruleCode: string | null }
  | { type: "UPDATE_PAYLOAD"; id?: string; payload: Partial<AnnotationPayload> }
  | { type: "SUBMIT_SUCCESS"; id?: string }
  | { type: "MARK_ORPHANED"; id?: string };
