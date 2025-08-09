import { useEffect } from "react";
import { useNavigate, useSearchParams } from "react-router";
import { useMockData } from "~/providers/mock-data-provider";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Loader2 } from "lucide-react";

export default function LegacySession() {
  const { createSession } = useMockData();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const topic = params.get("topic") || "Operating Systems";

  useEffect(() => {
    const id = createSession(topic);
    navigate(`/sessions/${id}`, { replace: true });
  }, [topic, createSession, navigate]);

  return (
    <div className="flex min-h-[50vh] items-center justify-center px-4">
      <Card>
        <CardHeader>
          <CardTitle>Preparing mocked session…</CardTitle>
          <CardDescription>
            Creating a local session for “{topic}”.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Please wait
        </CardContent>
      </Card>
    </div>
  );
}
