import { useEffect } from "react";
import { useNavigate, useSearchParams } from "react-router";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Loader2 } from "lucide-react";

export default function NewSession() {
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const topic = params.get("topic") || "Untitled Topic";

  // This component now simply navigates to the session detail page.
  // The session detail page is responsible for establishing the connection.
  useEffect(() => {
    // We use a random ID for the URL, as the real session is managed by the WebSocket.
    const id = Math.random().toString(36).slice(2, 10);
    const to = `/sessions/${id}?topic=${encodeURIComponent(topic)}`;
    const t = setTimeout(() => navigate(to, { replace: true }), 100);
    return () => clearTimeout(t);
  }, [topic, navigate]);

  return (
    <div className="flex min-h-[50vh] items-center justify-center">
      <Card>
        <CardHeader>
          <CardTitle>Starting session...</CardTitle>
          <CardDescription>
            Connecting to the Feynman agent for topic: "{topic}".
          </CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Redirecting
        </CardContent>
      </Card>
    </div>
  );
}
