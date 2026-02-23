"use client";

import { useState, useEffect } from "react";
import { ChevronDown, ChevronRight, Clock, Cpu, Hash, FileText, Layers } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardContent } from "@/components/ui/card";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { listAiExecutionLogs, listChecklistItems, type AiExecutionLog, type ChecklistItem } from "@/lib/api/prompt-lab";

export default function LogsPage() {
  const [logs, setLogs] = useState<AiExecutionLog[]>([]);
  const [checklistItems, setChecklistItems] = useState<ChecklistItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  useEffect(() => {
    Promise.all([
      listAiExecutionLogs({}),
      listChecklistItems({}),
    ]).then(([logsData, itemsData]) => {
      setLogs(logsData);
      setChecklistItems(itemsData);
      setLoading(false);
    });
  }, []);

  const toggleExpanded = (id: number) => {
    const next = new Set(expanded);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setExpanded(next);
  };

  const getChecklistName = (id: number) => {
    const item = checklistItems.find((i) => i.id === id);
    return item?.name || `Checklist Item #${id}`;
  };

  if (loading) {
    return <div className="p-8 text-center text-muted-foreground">Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-bold">Execution Logs</h1>

      <div className="grid gap-4">
        {logs.map((log) => (
          <Collapsible key={log.id} open={expanded.has(log.id)}>
            <Card className="hover:shadow-md transition-shadow">
              <CardHeader className="py-3">
                <CollapsibleTrigger asChild onClick={() => toggleExpanded(log.id)}>
                  <Button variant="ghost" className="w-full justify-between">
                    <div className="flex items-center gap-3">
                      {expanded.has(log.id) ? (
                        <ChevronDown className="h-4 w-4" />
                      ) : (
                        <ChevronRight className="h-4 w-4" />
                      )}
                      <span className="font-mono text-sm">Log #{log.id}</span>
                      <div className="flex items-center gap-1">
                        <Layers className="h-3 w-3 text-muted-foreground" />
                        <span className="text-xs">{log.context_type} #{log.context_id}</span>
                      </div>
                      <Badge variant="outline">{log.model_provider}</Badge>
                    </div>
                    <Badge
                      variant={
                        log.exec_status === "success"
                          ? "default"
                          : log.exec_status === "pending"
                          ? "secondary"
                          : "destructive"
                      }
                    >
                      {log.exec_status}
                    </Badge>
                  </Button>
                </CollapsibleTrigger>
              </CardHeader>
              <CollapsibleContent>
                <CardContent className="pt-0">
                  <div className="flex flex-wrap gap-4 text-sm mb-4">
                    <div className="flex items-center gap-2">
                      <FileText className="h-4 w-4 text-muted-foreground" />
                      <span className="text-muted-foreground">检查项:</span>
                      <span className="font-medium">{getChecklistName(log.check_item_id)}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Cpu className="h-4 w-4 text-muted-foreground" />
                      <span className="text-muted-foreground">Latency:</span>
                      <span className="tabular-nums">{log.latency_ms}ms</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Hash className="h-4 w-4 text-muted-foreground" />
                      <span className="text-muted-foreground">Input:</span>
                      <span className="tabular-nums">{log.input_tokens}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Hash className="h-4 w-4 text-muted-foreground" />
                      <span className="text-muted-foreground">Output:</span>
                      <span className="tabular-nums">{log.output_tokens}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Clock className="h-4 w-4 text-muted-foreground" />
                      <span className="text-muted-foreground">Time:</span>
                      <span className="tabular-nums">{new Date(log.created_at).toLocaleString()}</span>
                    </div>
                  </div>
                  {log.prompt_snapshot && (
                    <div className="space-y-2">
                      <h4 className="text-sm font-medium">Prompt Snapshot</h4>
                      <pre className="p-2 bg-muted rounded-md text-xs overflow-x-auto">
                        {log.prompt_snapshot}
                      </pre>
                    </div>
                  )}
                  {log.raw_output && (
                    <div className="space-y-2 mt-4">
                      <h4 className="text-sm font-medium">Raw Output</h4>
                      <pre className="p-2 bg-muted rounded-md text-xs overflow-x-auto">
                        {log.raw_output}
                      </pre>
                    </div>
                  )}
                  {log.error_message && (
                    <div className="mt-4 p-2 bg-destructive/10 rounded-md text-destructive text-sm">
                      {log.error_message}
                    </div>
                  )}
                </CardContent>
              </CollapsibleContent>
            </Card>
          </Collapsible>
        ))}
      </div>
    </div>
  );
}
