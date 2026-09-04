export interface StreamHandle {
  close(): void;
}

export interface StreamConnector<T> {
  connect(handlers: {
    onOpen: () => void;
    onMessage: (value: T) => void;
    onClose: () => void;
    onError: (error: unknown) => void;
  }): StreamHandle;
}

export interface ReconnectingStreamOptions<T> {
  connector: StreamConnector<T>;
  maxHistory?: number;
  dedupeKey?: (value: T) => string;
  onUpdate?: (history: readonly T[]) => void;
  onState?: (state: StreamState) => void;
  retryDelayMs?: number;
  maxReconnectAttempts?: number;
  schedule?: (retry: () => void, delayMs: number) => ReturnType<typeof setTimeout>;
  cancelSchedule?: (handle: ReturnType<typeof setTimeout>) => void;
}

export type StreamState = 'idle' | 'connecting' | 'live' | 'reconnecting' | 'closed' | 'cancelled';

/** Bounded history plus reconnect semantics shared by console and notifications. */
export class ReconnectingStream<T> {
  private readonly connector: StreamConnector<T>;
  private readonly maxHistory: number;
  private readonly keyFor: (value: T) => string;
  private readonly onUpdate?: ReconnectingStreamOptions<T>['onUpdate'];
  private readonly onState?: ReconnectingStreamOptions<T>['onState'];
  private readonly retryDelayMs: number;
  private readonly maxReconnectAttempts: number;
  private readonly schedule: NonNullable<ReconnectingStreamOptions<T>['schedule']>;
  private readonly cancelSchedule: NonNullable<ReconnectingStreamOptions<T>['cancelSchedule']>;
  private readonly history: T[] = [];
  private readonly seen = new Set<string>();
  private handle: StreamHandle | null = null;
  private stateValue: StreamState = 'idle';
  private terminal = false;
  private reconnectAttempts = 0;
  private retryHandle: ReturnType<typeof setTimeout> | null = null;

  constructor(options: ReconnectingStreamOptions<T>) {
    this.connector = options.connector;
    this.maxHistory = options.maxHistory ?? 200;
    this.keyFor = options.dedupeKey ?? ((value) => JSON.stringify(value));
    this.onUpdate = options.onUpdate;
    this.onState = options.onState;
    this.retryDelayMs = options.retryDelayMs ?? 500;
    this.maxReconnectAttempts = options.maxReconnectAttempts ?? 8;
    this.schedule = options.schedule ?? ((retry, delayMs) => setTimeout(retry, delayMs));
    this.cancelSchedule = options.cancelSchedule ?? ((handle) => clearTimeout(handle));
  }

  get state(): StreamState {
    return this.stateValue;
  }

  get historySnapshot(): readonly T[] {
    return structuredClone(this.history);
  }

  connect(): void {
    if (this.terminal || this.stateValue === 'live' || this.stateValue === 'connecting') return;
    this.setState(this.stateValue === 'idle' ? 'connecting' : 'reconnecting');
    this.handle = this.connector.connect({
      onOpen: () => {
        this.reconnectAttempts = 0;
        this.setState('live');
      },
      onMessage: (value) => this.receive(value),
      onClose: () => this.handleClosed(),
      onError: () => this.handleClosed(),
    });
  }

  receive(value: T): void {
    if (this.stateValue === 'connecting' || this.stateValue === 'reconnecting') {
      this.reconnectAttempts = 0;
      this.setState('live');
    }
    const key = this.keyFor(value);
    if (this.seen.has(key)) return;
    this.seen.add(key);
    this.history.push(structuredClone(value));
    while (this.history.length > this.maxHistory) {
      const removed = this.history.shift();
      if (removed !== undefined) this.seen.delete(this.keyFor(removed));
    }
    this.onUpdate?.(this.historySnapshot);
  }

  close(): void {
    this.terminal = true;
    this.cancelPendingRetry();
    this.handle?.close();
    this.handle = null;
    this.setState('closed');
  }

  cancel(): void {
    this.terminal = true;
    this.cancelPendingRetry();
    this.handle?.close();
    this.handle = null;
    this.setState('cancelled');
  }

  markTerminal(value?: T): void {
    if (value !== undefined) this.receive(value);
    this.terminal = true;
    this.cancelPendingRetry();
    this.handle?.close();
    this.handle = null;
    this.setState('closed');
  }

  private handleClosed(): void {
    this.handle = null;
    if (this.terminal) return;
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.setState('closed');
      return;
    }
    this.reconnectAttempts += 1;
    this.setState('reconnecting');
    this.retryHandle = this.schedule(() => {
      this.retryHandle = null;
      this.connect();
    }, this.retryDelayMs);
  }

  private cancelPendingRetry(): void {
    if (this.retryHandle !== null) this.cancelSchedule(this.retryHandle);
    this.retryHandle = null;
  }

  private setState(state: StreamState): void {
    this.stateValue = state;
    this.onState?.(state);
  }
}

export class BrowserWebSocketConnector<T> implements StreamConnector<T> {
  constructor(
    private readonly url: string,
    private readonly protocols?: string | string[],
  ) {}

  connect(handlers: Parameters<StreamConnector<T>['connect']>[0]): StreamHandle {
    const socket = new WebSocket(this.url, this.protocols);
    let ended = false;
    const fail = (error: unknown) => {
      if (ended) return;
      ended = true;
      handlers.onError(error);
      socket.close();
    };
    socket.onopen = () => handlers.onOpen();
    socket.onmessage = (event) => {
      try {
        handlers.onMessage(JSON.parse(String(event.data)) as T);
      } catch (error) {
        fail(error);
      }
    };
    socket.onclose = () => {
      if (ended) return;
      ended = true;
      handlers.onClose();
    };
    socket.onerror = () => fail(new Error('WebSocket error'));
    return {
      close: () => {
        if (ended) return;
        ended = true;
        socket.close();
      },
    };
  }
}
