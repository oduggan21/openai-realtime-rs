import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import CircularProgress from "./circular-progress";
import { Brain, Mic, AudioLines, Timer } from "lucide-react";
import { cn } from "@revlentless/ui/lib/utils";
import { type AIStatus } from "~/providers/feynman-provider";
import { useEffect, useState } from "react";

export default function SessionStatusPanel({
  aiStatus = "listening",
}: {
  aiStatus?: AIStatus;
}) {
  const [elapsedSec, setElapsedSec] = useState(0);
  const status = getStatus(aiStatus);
  const elapsed = formatTime(elapsedSec);

  useEffect(() => {
    const timer = setInterval(() => {
      setElapsedSec((s) => s + 1);
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  return (
    <aside aria-label="Session status and controls">
      <Card className="h-full">
        <CardHeader>
          <CardTitle className="text-base">Session Status</CardTitle>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="rounded-lg border p-3">
            <div className="text-sm text-muted-foreground">Session Time</div>
            <div className="mt-1 inline-flex items-center gap-2 text-2xl font-semibold tracking-tight">
              <Timer className="h-5 w-5 text-muted-foreground" />
              {elapsed}
            </div>
          </div>

          <div className="rounded-lg border p-3">
            <div className="mb-2 text-sm text-muted-foreground">
              Overall Progress
            </div>
            <div className="flex items-center gap-4">
              <CircularProgress value={0} size={72} strokeWidth={8} />
              <div className="text-sm text-muted-foreground">
                Progress tracking coming soon
              </div>
            </div>
          </div>

          <div className="rounded-lg border p-3">
            <div className="mb-2 text-sm text-muted-foreground">AI Status</div>
            <div className="flex items-center gap-3">
              <status.Icon
                className={cn(
                  "h-5 w-5",
                  status.key === "listening" && "text-emerald-600",
                  status.key === "thinking" && "text-purple-600",
                  status.key === "speaking" && "text-amber-600"
                )}
                aria-hidden="true"
              />
              <div className="text-sm">
                <div className="font-medium">{status.title}</div>
                <div className="text-muted-foreground">{status.subtitle}</div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </aside>
  );
}

function getStatus(s: AIStatus) {
  if (s === "thinking")
    return {
      key: s,
      title: "Analyzing your explanation...",
      subtitle: "Please wait a moment",
      Icon: Brain,
    };
  if (s === "speaking")
    return {
      key: s,
      title: "Asking a question...",
      subtitle: "Respond when you are ready",
      Icon: AudioLines,
    };
  return {
    key: "listening" as const,
    title: "Listening to you...",
    subtitle: "Ready for your input",
    Icon: Mic,
  };
}

function formatTime(sec: number) {
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
