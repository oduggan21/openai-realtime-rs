import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { FeynmanClient } from "~/lib/feynman-client";
import { toast } from "@revlentless/ui/components/sonner";

// State definitions
export type AIStatus = "listening" | "thinking" | "speaking";
export type ChatMessage = { id: string; role: "user" | "ai"; content: string };
export type Subtopic = { name: string };

// The shape of our context
type FeynmanContextType = {
  isConnected: boolean;
  aiStatus: AIStatus;
  mainTopic: string | null;
  subtopics: Subtopic[];
  messages: ChatMessage[];
  connect: (topic: string) => void;
  disconnect: () => void;
  sendUserMessage: (text: string) => void;
};

const FeynmanContext = createContext<FeynmanContextType | null>(null);

// The provider component
export function FeynmanProvider({ children }: { children: ReactNode }) {
  const [isConnected, setIsConnected] = useState(false);
  const [aiStatus, setAiStatus] = useState<AIStatus>("listening");
  const [mainTopic, setMainTopic] = useState<string | null>(null);
  const [subtopics, setSubtopics] = useState<Subtopic[]>([]);
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  const clientRef = useRef<FeynmanClient | null>(null);

  const connect = useCallback((topic: string) => {
    if (clientRef.current) {
      console.warn("Already connected or connecting.");
      return;
    }

    const wsUrl = import.meta.env.VITE_WS_URL || "ws://localhost:3000/ws";
    const client = new FeynmanClient(wsUrl);
    clientRef.current = client;

    client.on("open", () => {
      setIsConnected(true);
      toast.success("Connected to agent.");
    });

    client.on("initialized", (data) => {
      setMainTopic(data.mainTopic);
      setSubtopics(data.subtopics.map((name) => ({ name })));
      setMessages([]); // Clear messages for new session
      setAiStatus("listening");
      toast.info(`Session started for topic: ${data.mainTopic}`);
    });

    client.on("agentResponse", (data) => {
      setMessages((prev) => [
        ...prev,
        { id: crypto.randomUUID(), role: "ai", content: data.text },
      ]);
      setAiStatus("listening");
    });

    client.on("serverError", (error) => {
      toast.error("Server Error", { description: error.message });
      setAiStatus("listening");
    });

    client.on("close", () => {
      setIsConnected(false);
      setMainTopic(null);
      setSubtopics([]);
      setMessages([]);
      toast.warning("Disconnected from agent.");
    });

    client.on("error", () => {
      toast.error("Connection failed", {
        description: "Could not connect to the WebSocket server.",
      });
    });

    client.connect(topic);
  }, []);

  const disconnect = useCallback(() => {
    clientRef.current?.close();
    clientRef.current = null;
  }, []);

  const sendUserMessage = useCallback(
    (text: string) => {
      if (!clientRef.current || !isConnected) {
        toast.error("Not connected to the agent.");
        return;
      }
      clientRef.current.sendUserMessage(text);
      setMessages((prev) => [
        ...prev,
        { id: crypto.randomUUID(), role: "user", content: text },
      ]);
      setAiStatus("thinking");
    },
    [isConnected]
  );

  const value = useMemo(
    () => ({
      isConnected,
      aiStatus,
      mainTopic,
      subtopics,
      messages,
      connect,
      disconnect,
      sendUserMessage,
    }),
    [
      isConnected,
      aiStatus,
      mainTopic,
      subtopics,
      messages,
      connect,
      disconnect,
      sendUserMessage,
    ]
  );

  return (
    <FeynmanContext.Provider value={value}>{children}</FeynmanContext.Provider>
  );
}

// The custom hook for components to use
export function useFeynman() {
  const context = useContext(FeynmanContext);
  if (!context) {
    throw new Error("useFeynman must be used within a FeynmanProvider");
  }
  return context;
}
