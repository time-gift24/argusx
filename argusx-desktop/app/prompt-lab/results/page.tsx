"use client";

import { useState, useEffect } from "react";
import { CheckCircle, XCircle, Clock, FileText } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import {
  listCheckResults,
  listChecklistItems,
  listAiExecutionLogs,
  type CheckResult,
  type ChecklistItem,
  type AiExecutionLog,
} from "@/lib/api/prompt-lab";

export default function ResultsPage() {
  const [results, setResults] = useState<CheckResult[]>([]);
  const [checklistItems, setChecklistItems] = useState<ChecklistItem[]>([]);
  const [executionLogs, setExecutionLogs] = useState<AiExecutionLog[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      listCheckResults({}),
      listChecklistItems({}),
      listAiExecutionLogs({}),
    ]).then(([resultsData, itemsData, logsData]) => {
      setResults(resultsData);
      setChecklistItems(itemsData);
      setExecutionLogs(logsData);
      setLoading(false);
    });
  }, []);

  const getChecklistName = (checkItemId: number) => {
    const item = checklistItems.find((i) => i.id === checkItemId);
    return item?.name || `Checklist Item #${checkItemId}`;
  };

  const getExecutionLog = (resultId: number) => {
    return executionLogs.find((log) => log.check_result_id === resultId);
  };

  if (loading) {
    return <div className="p-8 text-center text-muted-foreground">Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-bold">Check Results</h1>

      <div className="grid gap-4">
        {results.map((result) => {
          const log = getExecutionLog(result.id);

          return (
            <Card key={result.id} className="hover:shadow-md transition-shadow">
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
                <div className="space-y-3">
                  {/* 检查项名称 */}
                  <div className="flex items-center gap-2 text-sm">
                    <FileText className="h-4 w-4 text-muted-foreground" />
                    <span className="text-muted-foreground">检查项:</span>
                    <span className="font-medium">{getChecklistName(result.check_item_id)}</span>
                  </div>

                  {/* AI 执行详情 */}
                  <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
                    <Badge variant="outline">{result.source_type}</Badge>
                    {log && (
                      <>
                        <div className="flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          <span>{log.latency_ms}ms</span>
                        </div>
                        <div>
                          Tokens: {log.input_tokens}/{log.output_tokens}
                        </div>
                        {log.model_version && (
                          <div className="text-xs">
                            模型: {log.model_version}
                          </div>
                        )}
                      </>
                    )}
                  </div>

                  {/* 时间 */}
                  <div className="text-xs text-muted-foreground pt-2 border-t">
                    {new Date(result.created_at).toLocaleString()}
                  </div>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
