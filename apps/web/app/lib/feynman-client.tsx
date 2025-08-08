// --- Type Definitions for WebSocket Protocol ---

/**
 * Defines the shape of messages sent from the client to the server.
 * This is a discriminated union based on the `type` property.
 */
type ClientToServerMessage =
  | {
      type: "init";
      topic: string;
    }
  | {
      type: "user_message";
      text: string;
    };

/**
 * Defines the shape of messages received from the server.
 * This is a discriminated union based on the `type` property.
 */
type ServerToClientMessage =
  | {
      type: "initialized";
      main_topic: string;
      subtopics: string[];
    }
  | {
      type: "agent_response";
      text: string;
    }
  | {
      type: "error";
      message: string;
    };

// --- Type Definitions for Client Events ---

/**
 * Defines the events that the FeynmanClient can emit, along with their payload types.
 * This allows for strongly-typed event listeners.
 */
interface FeynmanClientEvents {
  /** Fired when the WebSocket connection is successfully opened. */
  open: () => void;
  /** Fired when the WebSocket connection is closed. */
  close: (event: CloseEvent) => void;
  /** Fired when a connection error occurs. */
  error: (error: Event) => void;
  /** Fired when the server confirms the session is initialized with a topic. */
  initialized: (data: { mainTopic: string; subtopics: string[] }) => void;
  /** Fired when a new response is received from the agent. */
  agentResponse: (data: { text: string }) => void;
  /** Fired when the server reports a specific application-level error. */
  serverError: (data: { message: string }) => void;
}

/**
 * A robust, event-driven WebSocket client for the Feynman teaching agent API.
 */
export class FeynmanClient {
  private ws: WebSocket | null = null;
  private listeners: {
    [K in keyof FeynmanClientEvents]?: Array<FeynmanClientEvents[K]>;
  } = {};

  /**
   * Creates an instance of the FeynmanClient.
   * @param url The WebSocket URL of the Feynman server (e.g., "ws://localhost:3000/ws").
   */
  constructor(private url: string) {}

  /**
   * Registers an event handler for a specific event.
   * @param event The name of the event to listen for.
   * @param listener The callback function to execute when the event is fired.
   */
  public on<K extends keyof FeynmanClientEvents>(
    event: K,
    listener: FeynmanClientEvents[K]
  ): void {
    if (!this.listeners[event]) {
      this.listeners[event] = [];
    }
    this.listeners[event]?.push(listener);
  }

  /**
   * Removes a previously registered event handler.
   * @param event The name of the event.
   * @param listener The callback function to remove.
   */
  public off<K extends keyof FeynmanClientEvents>(
    event: K,
    listener: FeynmanClientEvents[K]
  ): void {
    const listenersForEvent = this.listeners[event];
    if (!listenersForEvent) {
      return;
    }

    // Find the index of the listener and remove it in-place with splice.
    // This avoids re-assigning the array, which solves the TypeScript error.
    const index = listenersForEvent.indexOf(listener);
    if (index > -1) {
      listenersForEvent.splice(index, 1);
    }
  }

  /**
   * Emits an event to all registered listeners.
   * @private
   */
  private emit<K extends keyof FeynmanClientEvents>(
    event: K,
    ...args: Parameters<FeynmanClientEvents[K]>
  ): void {
    if (!this.listeners[event]) {
      return;
    }
    // @ts-ignore - We know the arguments match the listener signature.
    this.listeners[event]?.forEach((listener) => listener(...args));
  }

  /**
   * Establishes a connection to the WebSocket server and initializes a new session.
   * @param topic The main topic for the Feynman teaching session.
   */
  public connect(topic: string): void {
    if (this.ws) {
      console.warn("FeynmanClient is already connected or connecting.");
      return;
    }

    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log("FeynmanClient: WebSocket connection opened.");
      // The first message must be 'init' to set up the session on the backend.
      this.sendMessageToServer({ type: "init", topic });
      this.emit("open");
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const message: ServerToClientMessage = JSON.parse(event.data);
        this.handleServerMessage(message);
      } catch (error) {
        console.error("FeynmanClient: Failed to parse server message.", error);
      }
    };

    this.ws.onclose = (event: CloseEvent) => {
      console.log(
        `FeynmanClient: WebSocket connection closed. Code: ${event.code}, Reason: ${event.reason}`
      );
      this.ws = null;
      this.emit("close", event);
    };

    this.ws.onerror = (error: Event) => {
      console.error("FeynmanClient: WebSocket error.", error);
      this.emit("error", error);
    };
  }

  /**
   * Handles incoming messages from the server and emits the appropriate client-side events.
   * @private
   */
  private handleServerMessage(message: ServerToClientMessage): void {
    switch (message.type) {
      case "initialized":
        this.emit("initialized", {
          mainTopic: message.main_topic,
          subtopics: message.subtopics,
        });
        break;
      case "agent_response":
        this.emit("agentResponse", { text: message.text });
        break;

      case "error":
        this.emit("serverError", { message: message.message });
        break;
    }
  }

  /**
   * Sends a message from the user to the agent.
   * @param text The user's message content.
   */
  public sendUserMessage(text: string): void {
    if (this.ws?.readyState !== WebSocket.OPEN) {
      console.error(
        "FeynmanClient: Cannot send message, WebSocket is not open."
      );
      return;
    }
    this.sendMessageToServer({ type: "user_message", text });
  }

  /**
   * Closes the WebSocket connection.
   */
  public close(): void {
    if (this.ws) {
      this.ws.close();
    }
  }

  /**
   * A private helper to stringify and send any client-to-server message.
   * @private
   */
  private sendMessageToServer(message: ClientToServerMessage): void {
    if (this.ws) {
      this.ws.send(JSON.stringify(message));
    }
  }
}
