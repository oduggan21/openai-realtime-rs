import { useEffect, useRef, useState } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Avatar, AvatarFallback } from "@revlentless/ui/components/avatar";
import { Bot, Paperclip, Send } from "lucide-react";
import { Button } from "@revlentless/ui/components/button";
import { Textarea } from "@revlentless/ui/components/textarea";
import { Separator } from "@revlentless/ui/components/separator";
import { useFeynman, type AIStatus } from "~/providers/feynman-provider";

export type ChatMessage = {
  id: string;
  role: "user" | "ai";
  content: string;
};

export default function ConversationView({
  messages = [] as ChatMessage[],
  aiStatus,
}: {
  messages?: ChatMessage[];
  aiStatus: AIStatus;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const { sendUserMessage } = useFeynman();
  const [inputText, setInputText] = useState("");

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [messages]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = inputText.trim();
    if (text) {
      sendUserMessage(text);
      setInputText("");
    }
  };

  return (
    <section aria-label="Conversation">
      <Card className="h-full">
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle className="text-base">Conversation</CardTitle>
        </CardHeader>

        <CardContent className="flex h-[70vh] flex-col sm:h-[72vh]">
          <div ref={scrollRef} className="flex-1 overflow-y-auto pr-1">
            <div className="space-y-3">
              {messages.map((m) =>
                m.role === "ai" ? (
                  <div key={m.id} className="flex items-start gap-3">
                    <Avatar className="h-8 w-8">
                      <AvatarFallback aria-label="AI">
                        <Bot className="h-4 w-4" aria-hidden="true" />
                      </AvatarFallback>
                    </Avatar>
                    <div className="max-w-[80%] rounded-2xl bg-muted px-3 py-2 text-sm">
                      {m.content}
                    </div>
                  </div>
                ) : (
                  <div key={m.id} className="flex justify-end">
                    <div className="max-w-[80%] rounded-2xl bg-emerald-600 px-3 py-2 text-sm text-white">
                      {m.content}
                    </div>
                  </div>
                )
              )}
            </div>
          </div>

          <div className="my-3">
            <Separator />
          </div>

          <form onSubmit={handleSubmit} className="rounded-lg border p-2">
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="icon" type="button">
                <Paperclip className="h-4 w-4" />
                <span className="sr-only">Attach</span>
              </Button>
              <Textarea
                placeholder="Type your explanation here..."
                className="min-h-[46px] resize-none"
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleSubmit(e);
                  }
                }}
                disabled={aiStatus !== "listening"}
              />
              <Button type="submit" disabled={aiStatus !== "listening"}>
                <Send className="mr-2 h-4 w-4" />
                Send
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </section>
  );
}
