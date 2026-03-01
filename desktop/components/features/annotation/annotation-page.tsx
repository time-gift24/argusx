"use client";

import { useEffect } from "react";
import { AnnotationWorkspace } from "./annotation-workspace";
import { useAnnotationStore } from "@/lib/stores/annotation-store";

export function AnnotationPage() {
  const loadCatalog = useAnnotationStore((store) => store.loadCatalog);

  useEffect(() => {
    loadCatalog();
  }, [loadCatalog]);

  return (
    <div className="flex flex-col gap-4">
      <header>
        <h1 className="text-xl font-semibold">LLM 标注工作台</h1>
        <p className="text-sm text-muted-foreground">
          左侧浏览审核文本，右侧维护单条标注。
        </p>
      </header>
      <AnnotationWorkspace />
    </div>
  );
}
