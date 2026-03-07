"use client";

import Link from "next/link";
import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { DEV_SHOWCASES } from "@/lib/dev-showcases";

export default function DevPage() {
  const defaultShowcase = DEV_SHOWCASES[0] ?? null;
  const [selectedId, setSelectedId] = useState(defaultShowcase?.id ?? "");
  const selected =
    DEV_SHOWCASES.find((item) => item.id === selectedId) ?? defaultShowcase;

  if (!selected) {
    return null;
  }

  return (
    <div className="grid flex-1 gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
      <section className="rounded-2xl border border-border/60 bg-card/70 p-3">
        <div className="space-y-1 border-b border-border/60 px-3 pb-4 pt-2">
          <p className="font-medium text-muted-foreground text-xs uppercase tracking-[0.2em]">
            Development
          </p>
          <h1 className="font-semibold text-2xl tracking-tight">Dev</h1>
          <p className="text-muted-foreground text-sm">
            Browse internal showcases before opening a focused playground.
          </p>
        </div>
        <div className="mt-3 space-y-1" role="list" aria-label="Dev showcases">
          {DEV_SHOWCASES.map((item) => (
            <button
              aria-label={item.title}
              aria-pressed={item.id === selected.id}
              className={cn(
                "flex w-full flex-col items-start gap-2 rounded-xl border px-3 py-3 text-left transition-colors",
                item.id === selected.id
                  ? "border-primary/30 bg-primary/8"
                  : "border-transparent hover:border-border/60 hover:bg-muted/40"
              )}
              data-state={item.id === selected.id ? "active" : "inactive"}
              key={item.id}
              onClick={() => setSelectedId(item.id)}
              type="button"
            >
              <div className="flex w-full items-center justify-between gap-3">
                <span className="font-medium text-sm">{item.title}</span>
                <Badge
                  className="shrink-0"
                  variant={item.status === "Available" ? "outline" : "secondary"}
                >
                  {item.status}
                </Badge>
              </div>
              <span className="text-muted-foreground text-xs leading-5">
                {item.summary}
              </span>
            </button>
          ))}
        </div>
      </section>
      <section className="rounded-2xl border border-border/60 bg-card p-6 lg:p-7">
        <div className="flex flex-wrap items-center gap-3">
          <h2 className="font-semibold text-2xl tracking-tight">
            {selected.title}
          </h2>
          <Badge variant={selected.status === "Available" ? "outline" : "secondary"}>
            {selected.status}
          </Badge>
        </div>
        <p className="mt-3 max-w-2xl text-muted-foreground text-sm leading-6">
          {selected.summary}
        </p>
        <p className="mt-5 max-w-3xl text-sm leading-7">{selected.details}</p>
        <div className="mt-6">
          <p className="font-medium text-muted-foreground text-xs uppercase tracking-[0.2em]">
            Highlights
          </p>
          <ul className="mt-3 flex flex-wrap gap-2">
            {selected.highlights.map((item) => (
              <li key={item}>
                <Badge variant="secondary">{item}</Badge>
              </li>
            ))}
          </ul>
        </div>
        <Link
          className="mt-8 inline-flex items-center rounded-md border border-border px-3 py-2 text-sm transition-colors hover:bg-muted"
          href={selected.href}
        >
          Open showcase
        </Link>
      </section>
    </div>
  );
}
