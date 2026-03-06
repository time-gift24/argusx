import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

type ChatModulePlaceholderProps = {
  variant: "page" | "sidebar";
};

export function ChatModulePlaceholder({ variant }: ChatModulePlaceholderProps) {
  if (variant === "sidebar") {
    return (
      <div className="flex min-h-0 flex-1 flex-col p-4">
        <Card className="border-dashed">
          <CardHeader className="gap-2">
            <Badge className="w-fit" variant="secondary">
              Placeholder
            </Badge>
            <CardTitle className="text-base">右侧面板占位</CardTitle>
            <CardDescription>此区域保留给后续重设计。</CardDescription>
          </CardHeader>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 items-start justify-start">
      <Card className="w-full max-w-6xl border-dashed">
        <CardHeader className="gap-3">
          <Badge className="w-fit" variant="secondary">
            Placeholder
          </Badge>
          <div className="space-y-1">
            <CardTitle className="text-2xl">对话模块已移除</CardTitle>
            <CardDescription className="text-sm">
              等待新的桌面工作台设计
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-muted-foreground">
          <p>当前保留路由和布局骨架，方便后续直接在原位置重建设计。</p>
          <p>这里不会再展示消息流、输入框、会话历史或模型配置。</p>
        </CardContent>
      </Card>
    </div>
  );
}
