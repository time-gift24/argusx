"use client";

import { useState, useEffect } from "react";
import { CheckCircle, XCircle, Folder } from "lucide-react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import {
  listChecklistItems,
  listGoldenSetItems,
  listCheckResults,
} from "@/lib/api/prompt-lab";

export default function PromptLabDashboard() {
  const [stats, setStats] = useState({
    checklistItems: 0,
    goldenSets: 1,
    passed: 0,
    failed: 0,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      listChecklistItems({}),
      listGoldenSetItems(1),
      listCheckResults({}),
    ]).then(([items, goldenItems, results]) => {
      setStats({
        checklistItems: items.length,
        goldenSets: goldenItems.length,
        passed: results.filter((r) => r.is_pass).length,
        failed: results.filter((r) => !r.is_pass).length,
      });
      setLoading(false);
    });
  }, []);

  const statCards = [
    { label: "Checklist Items", value: stats.checklistItems, icon: CheckCircle },
    { label: "Golden Sets", value: stats.goldenSets, icon: Folder },
    { label: "Passed", value: stats.passed, icon: CheckCircle },
    { label: "Failed", value: stats.failed, icon: XCircle },
  ];

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">PromptLab Dashboard</h1>
      <div className="grid gap-4 md:grid-cols-4">
        {statCards.map((stat) => (
          <Card key={stat.label}>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">{stat.label}</CardTitle>
              <stat.icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold tabular-nums">{stat.value}</div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
