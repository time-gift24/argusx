import { create } from "zustand";
import { annotationReducer, initialAnnotationState } from "@/lib/annotation/reducer";
import type { AnnotationAction, AnnotationState } from "@/lib/annotation/state";

type AnnotationStore = {
  state: AnnotationState;
  dispatch: (action: AnnotationAction) => void;
};

export const useAnnotationStore = create<AnnotationStore>((set) => ({
  state: initialAnnotationState,
  dispatch: (action) =>
    set((prev) => ({
      state: annotationReducer(prev.state, action),
    })),
}));
