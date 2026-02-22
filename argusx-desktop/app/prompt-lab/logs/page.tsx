"use client";

import { useState, useEffect } from "react";
import { ChevronDown, ChevronRight, Clock, Cpu, Hash } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { listAiExecutionLogs, type AiExecutionLog } from "@/lib/api/prompt-lab";

export default function LogsPage() {
  const [logs, setLogs] = useState<AiExecutionLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  useEffect(() => {
    listAiExecutionLogs({}).then((data) => {
      setLogs(data);
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

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-bold">Execution Logs</h1>

      <div className="grid gap-4">
        {logs.map((log) => (
          <Collapsible key={log.id} open={expanded.has(log.id)}>
            <Card>
              <CardHeader className="py-3">
                <CollapsibleTrigger asChild onClick={() => toggleExpanded(log.id)}>
                  <Button variant="ghost" className="w-full justify-between">
                    <div className="flex items-center gap-2">
                      {expanded.has(log.id) ? (
                        <ChevronDown className="h-4 w-4" />
                      ) : (
                        <ChevronRight className="h-4 w-4" />
                      )}
                      <span className="font-mono text-sm">Log #{log.id}</span>
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
                  <div className="grid grid-cols-4 gap-4 text-sm mb-4">
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
