import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Badge } from "@revlentless/ui/components/badge";
import { Progress } from "@revlentless/ui/components/progress";
import { Button } from "@revlentless/ui/components/button";
import {
  ArrowRight,
  Clock,
  BookOpen,
  MessageSquare,
  Sparkles,
} from "lucide-react";
import { useMockData } from "~/providers/mock-data-provider";
import { Link } from "react-router";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@revlentless/ui/components/table";

export default function Dashboard() {
  const { sessions, computeOverallPercent } = useMockData();
  const totalSessions = sessions.length;
  const activeSessions = sessions.filter((s) => s.status === "active").length;
  const avgProgress =
    sessions.length > 0
      ? Math.round(
          sessions
            .map((s) => computeOverallPercent(s.subtopics))
            .reduce((a, b) => a + b, 0) / sessions.length
        )
      : 0;

  const recent = sessions.slice(0, 5);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
          <p className="text-muted-foreground">
            A quick overview of your teaching sessions.
          </p>
        </div>
        <Button asChild className="bg-emerald-600 hover:bg-emerald-700">
          <Link to="/sessions/new?topic=Operating%20Systems">
            <Sparkles className="mr-2 h-4 w-4" />
            New Session
          </Link>
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Total Sessions
            </CardTitle>
            <BookOpen className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{totalSessions}</div>
            <p className="text-xs text-muted-foreground">
              All-time sessions (mocked)
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Active Sessions
            </CardTitle>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{activeSessions}</div>
            <p className="text-xs text-muted-foreground">
              Currently in progress
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Average Progress
            </CardTitle>
            <CardDescription>Across all sessions</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="mb-2 text-xl font-semibold">{avgProgress}%</div>
            <Progress value={avgProgress} className="h-2" />
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Recent Sessions</CardTitle>
            <CardDescription>Last 5 sessions you started</CardDescription>
          </CardHeader>
          <CardContent className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Topic</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Progress</TableHead>
                  <TableHead>Messages</TableHead>
                  <TableHead className="text-right">Action</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {recent.length === 0 ? (
                  <TableRow>
                    <TableCell
                      colSpan={5}
                      className="text-center text-muted-foreground"
                    >
                      No sessions yet. Start one!
                    </TableCell>
                  </TableRow>
                ) : (
                  recent.map((s) => (
                    <TableRow key={s.id}>
                      <TableCell className="font-medium">{s.topic}</TableCell>
                      <TableCell>
                        <Badge
                          variant={
                            s.status === "active" ? "default" : "secondary"
                          }
                        >
                          {s.status}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Progress
                            value={computeOverallPercent(s.subtopics)}
                            className="h-2 w-24"
                          />
                          <span className="text-xs text-muted-foreground">
                            {computeOverallPercent(s.subtopics)}%
                          </span>
                        </div>
                      </TableCell>
                      <TableCell className="whitespace-nowrap">
                        <div className="inline-flex items-center gap-1 text-muted-foreground">
                          <MessageSquare className="h-4 w-4" />
                          <span>{s.messages.length}</span>
                        </div>
                      </TableCell>
                      <TableCell className="text-right">
                        <Button asChild variant="outline" size="sm">
                          <Link to={`/sessions/${s.id}`}>
                            Open
                            <ArrowRight className="ml-2 h-4 w-4" />
                          </Link>
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Getting Started</CardTitle>
            <CardDescription>
              Tips to get the most out of your sessions
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="rounded-lg border p-3">
              <div className="font-medium">Teach in small chunks</div>
              <div className="text-sm text-muted-foreground">
                Pause briefly to let the AI ask better questions.
              </div>
            </div>
            <div className="rounded-lg border p-3">
              <div className="font-medium">Give concrete examples</div>
              <div className="text-sm text-muted-foreground">
                Use real-world scenarios to reinforce understanding.
              </div>
            </div>
            <div className="rounded-lg border p-3">
              <div className="font-medium">Reflect and iterate</div>
              <div className="text-sm text-muted-foreground">
                Revisit subtopics that received questions to improve
                comprehension.
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
