import Link from "next/link";
import { MessageCircle, Users, Settings, Sparkles, ArrowRight, Zap, Brain } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";

export default function DashboardPage() {
  return (
    <div className="flex flex-col gap-6 w-full">
      {/* Welcome Section */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-2xl">Welcome to ArgusX</CardTitle>
              <CardDescription className="mt-1">
                Your AI Agent platform with powerful LLM chat capabilities
              </CardDescription>
            </div>
            <Badge variant="secondary" className="flex items-center gap-1">
              <Sparkles className="h-3 w-3" />
              AI Ready
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Get started by creating a new chat or exploring your existing conversations.
            ArgusX provides a seamless experience for interacting with AI agents.
          </p>
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Quick Actions</h2>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <Link href="/chat" className="block">
            <Card className="hover:bg-accent/50 transition-colors cursor-pointer h-full">
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium">New Chat</CardTitle>
                <MessageCircle className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">Start</div>
                <p className="text-xs text-muted-foreground">
                  Create a new conversation
                </p>
              </CardContent>
            </Card>
          </Link>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Agents</CardTitle>
              <Users className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">0</div>
              <p className="text-xs text-muted-foreground">
                Active AI agents
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Templates</CardTitle>
              <Zap className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">3</div>
              <p className="text-xs text-muted-foreground">
                Pre-built templates
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Settings</CardTitle>
              <Settings className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">-</div>
              <p className="text-xs text-muted-foreground">
                Configure preferences
              </p>
            </CardContent>
          </Card>
        </div>
      </div>

      <Separator />

      {/* Stats Overview */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Overview</h2>
        <div className="grid gap-4 md:grid-cols-3">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Total Chats</CardTitle>
              <MessageCircle className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">0</div>
              <p className="text-xs text-muted-foreground">
                Conversations this month
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Messages</CardTitle>
              <Brain className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">0</div>
              <p className="text-xs text-muted-foreground">
                Total messages sent
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Active Time</CardTitle>
              <Zap className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">0h</div>
              <p className="text-xs text-muted-foreground">
                Time spent chatting
              </p>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Getting Started */}
      <Card>
        <CardHeader>
          <CardTitle>Getting Started</CardTitle>
          <CardDescription>
            Follow these steps to get the most out of ArgusX
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex items-center gap-4">
            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary text-primary-foreground text-sm font-medium">
              1
            </div>
            <div className="flex-1">
              <p className="font-medium">Start a new chat</p>
              <p className="text-sm text-muted-foreground">Begin a conversation with an AI agent</p>
            </div>
            <Button asChild variant="ghost" size="sm">
              <Link href="/chat">
                Go <ArrowRight className="ml-1 h-4 w-4" />
              </Link>
            </Button>
          </div>
          <Separator />
          <div className="flex items-center gap-4">
            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted text-sm font-medium">
              2
            </div>
            <div className="flex-1">
              <p className="font-medium">Configure your model</p>
              <p className="text-sm text-muted-foreground">Choose your preferred LLM provider</p>
            </div>
            <Button variant="ghost" size="sm" disabled>
              Coming soon
            </Button>
          </div>
          <Separator />
          <div className="flex items-center gap-4">
            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted text-sm font-medium">
              3
            </div>
            <div className="flex-1">
              <p className="font-medium">Create custom agents</p>
              <p className="text-sm text-muted-foreground">Build specialized AI agents for your needs</p>
            </div>
            <Button variant="ghost" size="sm" disabled>
              Coming soon
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
