import { create } from "zustand";
import { submitAnnotation, upsertAnnotationDraft } from "@/lib/api/annotation";
import { loadRuleCatalog } from "@/lib/annotation/loaders";
import { annotationReducer, initialAnnotationState } from "@/lib/annotation/reducer";
import type { AnnotationAction, AnnotationDraft, AnnotationState } from "@/lib/annotation/state";
import type { RuleCatalogItem } from "@/lib/annotation/types";

const AUTOSAVE_DELAY_MS = 500;
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleAutosave(draft: AnnotationDraft | undefined) {
  if (!draft || draft.status !== "draft") {
    return;
  }

  if (autosaveTimer) {
    clearTimeout(autosaveTimer);
  }

  autosaveTimer = setTimeout(() => {
    void upsertAnnotationDraft({
      id: draft.id,
      location: draft.location,
      ruleCode: draft.ruleCode,
      payload: draft.payload,
    });
  }, AUTOSAVE_DELAY_MS);
}

type AnnotationStore = {
  state: AnnotationState;
  catalog: RuleCatalogItem[];
  catalogSource: "remote" | "fallback" | null;
  dispatch: (action: AnnotationAction) => void;
  loadCatalog: (remoteFetcher?: () => Promise<RuleCatalogItem[]>) => Promise<void>;
  submitActive: () => Promise<void>;
};

export const useAnnotationStore = create<AnnotationStore>((set, get) => ({
  state: initialAnnotationState,
  catalog: [],
  catalogSource: null,

  dispatch: (action) => {
    const prevState = get().state;
    const prevActive = prevState.items.find((item) => item.id === prevState.activeId);

    set((prev) => ({
      state: annotationReducer(prev.state, action),
    }));

    if (action.type === "OPEN") {
      scheduleAutosave(prevActive);
      return;
    }

    if (action.type === "UPDATE_PAYLOAD" || action.type === "UPDATE_RULE") {
      const nextState = get().state;
      const nextActive = nextState.items.find((item) => item.id === nextState.activeId);
      scheduleAutosave(nextActive);
    }
  },

  loadCatalog: async (remoteFetcher) => {
    const catalog = await loadRuleCatalog(remoteFetcher);
    set({
      catalog: catalog.items,
      catalogSource: catalog.source,
    });
  },

  submitActive: async () => {
    const current = get().state;
    const active = current.items.find((item) => item.id === current.activeId);
    if (!active) {
      return;
    }

    await submitAnnotation(active.id);
    set((prev) => ({
      state: annotationReducer(prev.state, {
        type: "SUBMIT_SUCCESS",
        id: active.id,
      }),
    }));
  },
}));
