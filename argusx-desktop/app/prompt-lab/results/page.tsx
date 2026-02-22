"use client";

import { useState, useEffect } from "react";
import { CheckCircle, XCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { listCheckResults, type CheckResult } from "@/lib/api/prompt-lab";

export default function ResultsPage() {
  const [results, setResults] = useState<CheckResult[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    listCheckResults({}).then((data) => {
      setResults(data);
      setLoading(false);
    });
  }, []);

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-bold">Check Results</h1>

      <div className="grid gap-4">
        {results.map((result) => (
          <Card key={result.id}>
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle className="text-base">
                Context: {result.context_type} #{result.context_id}
              </CardTitle>
              <div className="flex items-center gap-2">
                {result.is_pass ? (
                  <CheckCircle className="h-5 w-5 text-green-500" aria-label="Passed" />
                ) : (
                  <XCircle className="h-5 w-5 text-red-500" aria-label="Failed" />
                )}
                <Badge variant={result.is_pass ? "default" : "destructive"}>
                  {result.is_pass ? "Passed" : "Failed"}
                </Badge>
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground">Check Item ID:</span>{" "}
                  <span className="tabular-nums">{result.check_item_id}</span>
                </div>
                <div>
                  <span className="text-muted-foreground">Source:</span>{" "}
                  <Badge variant="outline">{result.source_type}</Badge>
                </div>
                <div>
                  <span className="text-muted-foreground">Created:</span>{" "}
                  <span className="tabular-nums">{new Date(result.created_at).toLocaleString()}</span>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
