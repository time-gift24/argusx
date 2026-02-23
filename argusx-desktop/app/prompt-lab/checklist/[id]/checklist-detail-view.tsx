"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { ArrowLeft, Calendar } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { getChecklistItem, listCheckResults, type ChecklistItem, type CheckResult } from "@/lib/api/prompt-lab";

export function ChecklistDetailView({ id }: { id: string }) {
  const router = useRouter();
  const numericId = Number(id);

  const [item, setItem] = useState<ChecklistItem | null>(null);
  const [results, setResults] = useState<CheckResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Validate ID
  if (isNaN(numericId)) {
    return (
      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={() => router.back()}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back
          </Button>
        </div>
        <div className="text-center text-muted-foreground py-8">
          Invalid checklist item ID
        </div>
      </div>
    );
  }

  useEffect(() => {
    Promise.all([
      getChecklistItem(numericId),
      listCheckResults({ check_item_id: numericId }),
    ]).then(([itemData, resultsData]) => {
      setItem(itemData);
      setResults(resultsData);
      setLoading(false);
    }).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to load checklist item");
      setLoading(false);
    });
  }, [numericId]);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[200px]">
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={() => router.back()}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back
          </Button>
        </div>
        <div className="text-center text-destructive py-8">
          {error}
        </div>
      </div>
    );
  }

  if (!item) {
    return (
      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={() => router.back()}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back
          </Button>
        </div>
        <div className="text-center text-muted-foreground py-8">
          Checklist item not found
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="sm" onClick={() => router.back()}>
          <ArrowLeft className="h-4 w-4 mr-2" />
          Back
        </Button>
        <h1 className="text-2xl font-bold">Checklist Item</h1>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle className="text-xl">{item.name}</CardTitle>
            <div className="flex gap-2">
              <Badge variant={item.status === "active" ? "default" : "secondary"}>
                {item.status}
              </Badge>
              <Badge variant="outline">{item.target_level}</Badge>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <h3 className="text-sm font-medium mb-2">Prompt</h3>
            <pre className="p-3 bg-muted rounded-md overflow-auto max-h-60 text-sm whitespace-pre-wrap">
              {item.prompt}
            </pre>
          </div>

          {item.result_schema && (
            <div>
              <h3 className="text-sm font-medium mb-2">Result Schema</h3>
              <pre className="p-3 bg-muted rounded-md text-sm">
                {JSON.stringify(item.result_schema, null, 2)}
              </pre>
            </div>
          )}

          <div className="flex gap-4 text-sm text-muted-foreground">
            <span>Version: {item.version}</span>
          </div>

          <div className="flex gap-4 text-sm text-muted-foreground">
            <div className="flex items-center gap-1">
              <Calendar className="h-4 w-4" />
              <span>Created: {new Date(item.created_at).toLocaleString()}</span>
            </div>
            <div className="flex items-center gap-1">
              <Calendar className="h-4 w-4" />
              <span>Updated: {new Date(item.updated_at).toLocaleString()}</span>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Check History</CardTitle>
        </CardHeader>
        <CardContent>
          {results.length === 0 ? (
            <p className="text-muted-foreground">No check history yet.</p>
          ) : (
            <ul className="space-y-2">
              {results.slice(0, 5).map((result) => (
                <li
                  key={result.id}
                  className="flex items-center justify-between p-2 rounded-md bg-muted"
                >
                  <div className="flex items-center gap-2">
                    <Badge variant={result.is_pass ? "default" : "destructive"}>
                      {result.is_pass ? "Pass" : "Fail"}
                    </Badge>
                    <span className="text-sm">{result.source_type}</span>
                  </div>
                  <span className="text-sm text-muted-foreground">
                    {new Date(result.created_at).toLocaleString()}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Associated Golden Sets</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">
            Golden Set association feature coming soon.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
