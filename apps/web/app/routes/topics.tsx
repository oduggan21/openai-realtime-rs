import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@revlentless/ui/components/accordion";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Progress } from "@revlentless/ui/components/progress";
import { Badge } from "@revlentless/ui/components/badge";
import { useMockData } from "~/providers/mock-data-provider";
import { BookOpen, ListChecks } from "lucide-react";

export default function Topics() {
  const { topicsSummary } = useMockData();
  const topics = topicsSummary();

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Topics</h1>
          <p className="text-muted-foreground">
            Your most taught topics across sessions.
          </p>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {topics.map((t) => (
          <Card key={t.topic}>
            <CardHeader className="space-y-1">
              <CardTitle className="flex items-center gap-2 text-base">
                <BookOpen className="h-4 w-4 text-emerald-600" />
                {t.topic}
              </CardTitle>
              <CardDescription className="flex items-center gap-2">
                <ListChecks className="h-4 w-4" />
                {t.sessions} sessions
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">Avg. Progress</span>
                <span className="font-medium">{t.avgProgress}%</span>
              </div>
              <Progress value={t.avgProgress} className="h-2" />
              <Accordion type="single" collapsible className="w-full">
                <AccordionItem value="details">
                  <AccordionTrigger className="text-sm">
                    Curriculum details
                  </AccordionTrigger>
                  <AccordionContent>
                    <div className="flex flex-wrap gap-2">
                      {t.criteria.map((c) => (
                        <Badge key={c.label} variant={c.variant as any}>
                          {c.label}: {c.percent}%
                        </Badge>
                      ))}
                    </div>
                  </AccordionContent>
                </AccordionItem>
              </Accordion>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
