export interface ConsoleLineDTO {
  level?: string;
  source: string;
  text: string;
  ts: string;
}

function copy<T>(value: T): T {
  return structuredClone(value);
}

/** Models the contract's bounded-history-then-live delivery model without a clock. */
export class FakeWebSocket<T> {
  private connected = false;
  private readonly events: T[];
  private readonly historyLimit: number;

  constructor(history: T[] = [], historyLimit = 200) {
    this.events = history.map(copy);
    this.historyLimit = historyLimit;
    this.trimHistory();
  }

  get connectionCount(): number {
    return this.connections;
  }

  get live(): boolean {
    return this.connected;
  }

  private connections = 0;

  connect(): T[] {
    this.connected = true;
    this.connections += 1;
    return this.events.map(copy);
  }

  disconnect(): void {
    this.connected = false;
  }

  push(event: T): T[] {
    this.events.push(copy(event));
    this.trimHistory();
    return this.connected ? [copy(event)] : [];
  }

  reconnect(): T[] {
    this.disconnect();
    return this.connect();
  }

  private trimHistory(): void {
    this.events.splice(0, Math.max(0, this.events.length - this.historyLimit));
  }
}
