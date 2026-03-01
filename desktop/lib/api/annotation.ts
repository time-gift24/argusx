import type { AnnotationDraft } from "@/lib/annotation/state";
import type { RuleCatalogItem } from "@/lib/annotation/types";

export async function fetchRuleCatalog(): Promise<RuleCatalogItem[]> {
  const response = await fetch("/api/annotation/rules");
  if (!response.ok) {
    throw new Error(`failed to fetch rule catalog: ${response.status}`);
  }

  const data = (await response.json()) as RuleCatalogItem[];
  return data;
}

export async function fetchAnnotations(_docId: string): Promise<AnnotationDraft[]> {
  return [];
}

export async function upsertAnnotationDraft(payload: {
  id: string;
  location: AnnotationDraft["location"];
  ruleCode: string | null;
  payload: AnnotationDraft["payload"];
}): Promise<{ id: string; status: "draft" }> {
  return {
    id: payload.id,
    status: "draft",
  };
}

export async function submitAnnotation(id: string): Promise<{ id: string; status: "submitted" }> {
  return {
    id,
    status: "submitted",
  };
}
