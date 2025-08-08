import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { BookCheck } from "lucide-react";

// The backend currently sends a simple list of strings.
// We'll display them without progress tracking for now.
export type Subtopic = {
  name: string;
};

export default function SubtopicsPanel({
  topic = "Topic",
  subtopics = [] as Subtopic[],
}) {
  return (
    <section aria-label="Curriculum subtopics">
      <Card className="h-full">
        <CardHeader>
          <CardTitle className="text-base">Curriculum for {topic}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {subtopics.map((s) => (
            <div
              key={s.name}
              className="flex items-center gap-3 rounded-lg border p-3 text-sm"
            >
              <BookCheck className="h-4 w-4 text-muted-foreground" />
              <span className="font-medium">{s.name}</span>
            </div>
          ))}
        </CardContent>
      </Card>
    </section>
  );
}
