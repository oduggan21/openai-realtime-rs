import { Link, useNavigate, useParams, useSearchParams } from "react-router";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@revlentless/ui/components/breadcrumb";
import SubtopicsPanel from "~/components/session/subtopics-panel";
import ConversationView from "~/components/session/conversation-view";
import SessionStatusPanel from "~/components/session/session-status-panel";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@revlentless/ui/components/alert-dialog";
import { Button } from "@revlentless/ui/components/button";
import { Separator } from "@revlentless/ui/components/separator";
import { useFeynman } from "~/providers/feynman-provider";
import { useEffect } from "react";
import { Loader2 } from "lucide-react";

export default function SessionDetail() {
  const { id } = useParams();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const {
    connect,
    disconnect,
    isConnected,
    mainTopic,
    subtopics,
    messages,
    aiStatus,
  } = useFeynman();
  const topicFromUrl = params.get("topic") || "Untitled Topic";

  useEffect(() => {
    // Connect to the WebSocket when the component mounts
    connect(topicFromUrl);

    // Disconnect when the component unmounts
    return () => {
      disconnect();
    };
  }, [topicFromUrl, connect, disconnect]);

  if (!isConnected || !mainTopic) {
    return (
      <div className="flex min-h-[50vh] items-center justify-center">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Connecting to agent for topic: "{topicFromUrl}"...
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-[1400px]">
      <div className="mb-2 hidden lg:block">
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              <BreadcrumbLink asChild>
                <Link to="/sessions">Sessions</Link>
              </BreadcrumbLink>
            </BreadcrumbItem>
            <BreadcrumbSeparator />
            <BreadcrumbItem>
              <BreadcrumbPage>{mainTopic}</BreadcrumbPage>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      </div>

      <div className="mb-2 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">{mainTopic}</h1>
          <p className="text-sm text-muted-foreground">
            Teach the topic in your own words. The AI will ask questions as your
            curious student.
          </p>
        </div>
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button
              variant="outline"
              className="border-red-500 text-red-600 hover:bg-red-50"
            >
              End Session
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>End session?</AlertDialogTitle>
              <AlertDialogDescription>
                This will disconnect you from the agent. You can start a new
                session at any time.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction
                className="bg-red-600 hover:bg-red-700"
                onClick={() => {
                  disconnect();
                  navigate("/sessions");
                }}
              >
                End session
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>

      <Separator className="mb-4" />

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[320px_minmax(0,1fr)_360px]">
        <SubtopicsPanel topic={mainTopic} subtopics={subtopics} />
        <ConversationView messages={messages} aiStatus={aiStatus} />
        <SessionStatusPanel aiStatus={aiStatus} />
      </div>
    </div>
  );
}
