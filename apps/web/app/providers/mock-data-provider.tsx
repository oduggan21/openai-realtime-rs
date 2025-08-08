import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import type { ChatMessage } from "~/components/session/conversation-view";
import type { Subtopic } from "~/components/session/subtopics-panel";
import { toast } from "@revlentless/ui/components/sonner";

type SessionStatus = "active" | "ended";
export type AIStatus = "listening" | "thinking" | "speaking";

export type Session = {
  id: string;
  topic: string;
  createdAt: number;
  status: SessionStatus;
  messages: ChatMessage[];
  subtopics: Subtopic[];
  aiStatus: AIStatus;
  liveTranscript: string;
  elapsedSec: number;
};

type Store = {
  sessions: Session[];
  createSession: (topic: string) => string;
  endSession: (id: string) => void;
  appendUserMessage: (id: string, content: string) => void;
  appendAIMessage: (id: string, content: string) => void;
  startListening: (id: string) => void;
  stopListening: (id: string) => void;
  computeOverallPercent: (subtopics: Subtopic[]) => number;
  topicsSummary: () => {
    topic: string;
    sessions: number;
    avgProgress: number;
    criteria: {
      label: string;
      percent: number;
      variant: "secondary" | "default" | "destructive";
    }[];
  }[];
};

const MockDataContext = createContext<Store | null>(null);

const wordsPool =
  "the scheduler selects next process based on priority time slice io wait mutex semaphore paging segmentation tlb cache locality system call fork exec pipe interrupt context switch round robin multilevel feedback queue".split(
    " "
  );

const initialSubtopics: Subtopic[] = [
  {
    id: "proc",
    name: "Process Management",
    states: {
      Definition: "covered",
      Mechanism: "covered",
      Example: "questioned",
    },
  },
  {
    id: "mem",
    name: "Memory Management",
    states: { Definition: "covered", Mechanism: "pending", Example: "pending" },
  },
  {
    id: "conc",
    name: "Concurrency & Synchronization",
    states: {
      Definition: "questioned",
      Mechanism: "pending",
      Example: "pending",
    },
  },
  {
    id: "fs",
    name: "File Systems",
    states: { Definition: "covered", Mechanism: "covered", Example: "covered" },
  },
  {
    id: "io",
    name: "I/O Management",
    states: { Definition: "pending", Mechanism: "pending", Example: "pending" },
  },
  {
    id: "sched",
    name: "Scheduling",
    states: {
      Definition: "covered",
      Mechanism: "questioned",
      Example: "pending",
    },
  },
];

const initialMessages: ChatMessage[] = [
  {
    id: "m1",
    role: "user",
    content:
      "An operating system manages hardware resources and provides services to applications.",
  },
  {
    id: "m2",
    role: "ai",
    content:
      "Great start! Can you give a concrete example of a system call and what it does?",
  },
];

export function MockDataProvider({ children }: { children: ReactNode }) {
  const [sessions, setSessions] = useState<Session[]>(() => {
    if (typeof window === "undefined") return [];
    const raw = localStorage.getItem("feynman:sessions");
    if (raw) {
      try {
        return JSON.parse(raw) as Session[];
      } catch {}
    }
    return [];
  });

  const timersRef = useRef<Map<string, number>>(new Map());
  const tickerRef = useRef<number | null>(null);

  useEffect(() => {
    if (typeof window === "undefined") return;
    localStorage.setItem("feynman:sessions", JSON.stringify(sessions));
  }, [sessions]);

  useEffect(() => {
    if (tickerRef.current) window.clearInterval(tickerRef.current);
    tickerRef.current = window.setInterval(() => {
      setSessions((prev) =>
        prev.map((s) =>
          s.status === "active" ? { ...s, elapsedSec: s.elapsedSec + 1 } : s
        )
      );
    }, 1000);
    return () => {
      if (tickerRef.current) window.clearInterval(tickerRef.current);
    };
  }, []);

  const computeOverallPercent = useCallback((subs: Subtopic[]) => {
    const criteria: (keyof Subtopic["states"])[] = [
      "Definition",
      "Mechanism",
      "Example",
    ];
    let covered = 0;
    const total = subs.length * criteria.length;
    for (const s of subs)
      for (const c of criteria) if (s.states[c] === "covered") covered++;
    return total > 0 ? Math.round((covered / total) * 100) : 0;
  }, []);

  const createSession = useCallback((topic: string) => {
    const id = Math.random().toString(36).slice(2, 10);
    const session: Session = {
      id,
      topic,
      createdAt: Date.now(),
      status: "active",
      messages: initialMessages,
      subtopics: initialSubtopics,
      aiStatus: "listening",
      liveTranscript: "",
      elapsedSec: 0,
    };
    setSessions((prev) => [session, ...prev]);
    toast("Session created", { description: `Topic: ${topic}` });
    return id;
  }, []);

  const endSession = useCallback((id: string) => {
    setSessions((prev) =>
      prev.map((s) => (s.id === id ? { ...s, status: "ended" } : s))
    );
    const t = timersRef.current.get(id);
    if (t) window.clearInterval(t);
    timersRef.current.delete(id);
    toast("Session ended", { description: `ID: ${id}` });
  }, []);

  const appendUserMessage = useCallback((id: string, content: string) => {
    setSessions((prev) =>
      prev.map((s) =>
        s.id === id
          ? {
              ...s,
              messages: [
                ...s.messages,
                { id: cryptoId(), role: "user", content },
              ],
            }
          : s
      )
    );
  }, []);

  const appendAIMessage = useCallback((id: string, content: string) => {
    setSessions((prev) =>
      prev.map((s) =>
        s.id === id
          ? {
              ...s,
              messages: [
                ...s.messages,
                { id: cryptoId(), role: "ai", content },
              ],
            }
          : s
      )
    );
  }, []);

  const startListening = useCallback((id: string) => {
    setSessions((prev) =>
      prev.map((s) => (s.id === id ? { ...s, aiStatus: "listening" } : s))
    );
    const existing = timersRef.current.get(id);
    if (existing) window.clearInterval(existing);
    const t = window.setInterval(() => {
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== id) return s;
          const w = wordsPool[Math.floor(Math.random() * wordsPool.length)];
          return {
            ...s,
            liveTranscript: s.liveTranscript ? s.liveTranscript + " " + w : w,
          };
        })
      );
    }, 300);
    timersRef.current.set(id, t);
  }, []);

  const stopListening = useCallback(
    (id: string) => {
      const t = timersRef.current.get(id);
      if (t) window.clearInterval(t);
      timersRef.current.delete(id);

      let finalText = "";
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== id) return s;
          finalText = s.liveTranscript.trim();
          return { ...s, liveTranscript: "", aiStatus: "thinking" };
        })
      );
      if (finalText) appendUserMessage(id, finalText);

      setTimeout(() => {
        setSessions((prev) =>
          prev.map((s) => (s.id === id ? { ...s, aiStatus: "speaking" } : s))
        );
        setTimeout(() => {
          appendAIMessage(
            id,
            "Could you provide a concrete example that illustrates the mechanism you described?"
          );
          setSessions((prev) =>
            prev.map((s) => {
              if (s.id !== id) return s;
              const idx = Math.floor(Math.random() * s.subtopics.length);
              const keys: (keyof Subtopic["states"])[] = [
                "Definition",
                "Mechanism",
                "Example",
              ];
              const key = keys[Math.floor(Math.random() * keys.length)];
              const cur = s.subtopics[idx].states[key];
              const next =
                cur === "covered"
                  ? "questioned"
                  : cur === "questioned"
                  ? "covered"
                  : "questioned";
              const subtopics = s.subtopics.map((st, i) =>
                i === idx
                  ? { ...st, states: { ...st.states, [key]: next } }
                  : st
              );
              return { ...s, subtopics, aiStatus: "listening" };
            })
          );
        }, 800);
      }, 800);
    },
    [appendAIMessage, appendUserMessage]
  );

  const value = useMemo<Store>(
    () => ({
      sessions,
      createSession,
      endSession,
      appendUserMessage,
      appendAIMessage,
      startListening,
      stopListening,
      computeOverallPercent,
      topicsSummary: () => {
        const map = new Map<
          string,
          {
            count: number;
            totalPct: number;
            criteriaCovered: Record<string, { covered: number; total: number }>;
          }
        >();
        for (const s of sessions) {
          const pct = computeOverallPercent(s.subtopics);
          if (!map.has(s.topic)) {
            map.set(s.topic, {
              count: 0,
              totalPct: 0,
              criteriaCovered: {
                Definition: { covered: 0, total: 0 },
                Mechanism: { covered: 0, total: 0 },
                Example: { covered: 0, total: 0 },
              },
            });
          }
          const rec = map.get(s.topic)!;
          rec.count++;
          rec.totalPct += pct;
          for (const st of s.subtopics) {
            for (const k of ["Definition", "Mechanism", "Example"] as const) {
              rec.criteriaCovered[k].total++;
              if (st.states[k] === "covered") rec.criteriaCovered[k].covered++;
            }
          }
        }
        return Array.from(map.entries()).map(([topic, rec]) => ({
          topic,
          sessions: rec.count,
          avgProgress: rec.count ? Math.round(rec.totalPct / rec.count) : 0,
          criteria: (["Definition", "Mechanism", "Example"] as const).map(
            (k) => {
              const percent =
                rec.criteriaCovered[k].total > 0
                  ? Math.round(
                      (rec.criteriaCovered[k].covered /
                        rec.criteriaCovered[k].total) *
                        100
                    )
                  : 0;
              return {
                label: k,
                percent,
                variant:
                  percent > 66
                    ? "default"
                    : percent > 33
                    ? "secondary"
                    : "destructive",
              };
            }
          ),
        }));
      },
    }),
    [
      sessions,
      createSession,
      endSession,
      appendUserMessage,
      appendAIMessage,
      startListening,
      stopListening,
      computeOverallPercent,
    ]
  );

  return (
    <MockDataContext.Provider value={value}>
      {children}
    </MockDataContext.Provider>
  );
}

export function useMockData() {
  const ctx = useContext(MockDataContext);
  if (!ctx) throw new Error("useMockData must be used within MockDataProvider");
  return ctx;
}

function cryptoId() {
  return Math.random().toString(36).slice(2, 10);
}
