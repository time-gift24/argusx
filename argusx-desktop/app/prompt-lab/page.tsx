"use client";

import { useState, useEffect } from "react";
import { CheckCircle, XCircle, Folder, FileText, ArrowRight } from "lucide-react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  listChecklistItems,
  listGoldenSetItems,
  listCheckResults,
  listAiExecutionLogs,
} from "@/lib/api/prompt-lab";

interface ModuleStats {
  checklist: number;
  goldenSets: number;
  results: { passed: number; failed: number };
  logs: number;
}

const modules = [
  { key: "checklist", name: "Checklist", icon: CheckCircle, href: "/prompt-lab/checklist" },
  { key: "goldenSets", name: "Golden Sets", icon: Folder, href: "/prompt-lab/golden-sets" },
  { key: "results", name: "Results", icon: FileText, href: "/prompt-lab/results" },
  { key: "logs", name: "Logs", icon: FileText, href: "/prompt-lab/logs" },
];

export default function PromptLabDashboard() {
  const [stats, setStats] = useState<ModuleStats>({
    checklist: 0,
    goldenSets: 0,
    results: { passed: 0, failed: 0 },
    logs: 0,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      listChecklistItems({}),
      listGoldenSetItems(1),
      listCheckResults({}),
      listAiExecutionLogs({}),
    ]).then(([items, goldenItems, results, logs]) => {
      setStats({
        checklist: items.length,
        goldenSets: goldenItems.length,
        results: {
          passed: results.filter((r) => r.is_pass).length,
          failed: results.filter((r) => !r.is_pass).length,
        },
        logs: logs.length,
      });
      setLoading(false);
    });
  }, []);

  const getModuleValue = (key: string) => {
    switch (key) {
      case "checklist": return stats.checklist;
      case "goldenSets": return stats.goldenSets;
      case "results": return stats.results.passed + stats.results.failed;
      case "logs": return stats.logs;
      default: return 0;
    }
  };

  const getModuleSubtitle = (key: string) => {
    switch (key) {
      case "checklist": return "items";
      case "goldenSets": return "sets";
      case "results": return `${stats.results.passed} passed, ${stats.results.failed} failed`;
      case "logs": return "entries";
      default: return "";
    }
  };

  if (loading) {
    return <div className="p-8 text-center text-muted-foreground">Loading...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">PromptLab</h1>
      </div>

      {/* 九宫格模块卡片 */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {modules.map((module) => (
          <Card key={module.key} className="hover:shadow-md transition-shadow cursor-pointer group">
            <CardHeader className="flex flex-row items-center gap-3 pb-2">
              <div className="p-2 rounded-lg bg-primary/10">
                <module.icon className="h-5 w-5 text-primary" />
              </div>
              <CardTitle className="text-base font-semibold">{module.name}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                <div className="text-3xl font-bold tabular-nums">{getModuleValue(module.key)}</div>
                <div className="text-sm text-muted-foreground">{getModuleSubtitle(module.key)}</div>
                <Button variant="ghost" size="sm" className="w-full opacity-0 group-hover:opacity-100 transition-opacity">
                  查看详情 <ArrowRight className="h-4 w-4 ml-1" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* 数据流向可视化 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">数据流向</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center gap-2 md:gap-4 text-sm flex-wrap">
            <div className="flex items-center gap-2">
              <span className="px-3 py-1.5 rounded-md bg-primary/10 text-primary font-medium">
                Checklist ({stats.checklist})
              </span>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </div>
            <div className="flex items-center gap-2">
              <span className="px-3 py-1.5 rounded-md bg-primary/10 text-primary font-medium">
                Golden Sets ({stats.goldenSets})
              </span>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </div>
            <div className="flex items-center gap-2">
              <span className="px-3 py-1.5 rounded-md bg-green-500/10 text-green-600 font-medium">
                Passed ({stats.results.passed})
              </span>
              <span className="px-3 py-1.5 rounded-md bg-red-500/10 text-red-600 font-medium">
                Failed ({stats.results.failed})
              </span>
            </div>
            <div className="flex items-center gap-2">
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
              <span className="px-3 py-1.5 rounded-md bg-muted text-muted-foreground font-medium">
                Logs ({stats.logs})
              </span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
