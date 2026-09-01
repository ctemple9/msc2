export type RunningSnapshot = { running: boolean };

export type RunningStatePollOptions = {
  intervalMs?: number;
  timeoutMs?: number;
  now?: () => number;
  sleep?: (milliseconds: number) => Promise<void>;
};

const DEFAULT_INTERVAL_MS = 100;
const DEFAULT_TIMEOUT_MS = 10_000;

/**
 * Waits for the agent to report the state requested by a lifecycle action.
 * Start and stop are asynchronous, so the first status response can still
 * describe the previous process state.
 */
export async function waitForRunningState<T extends RunningSnapshot>(
  read: () => Promise<T>,
  desiredRunning: boolean,
  options: RunningStatePollOptions = {},
): Promise<T> {
  const intervalMs = options.intervalMs ?? DEFAULT_INTERVAL_MS;
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const now = options.now ?? Date.now;
  const sleep =
    options.sleep ??
    ((milliseconds: number) =>
      new Promise<void>((resolve) => globalThis.setTimeout(resolve, milliseconds)));
  const deadline = now() + timeoutMs;

  let snapshot = await read();
  while (snapshot.running !== desiredRunning && now() < deadline) {
    await sleep(intervalMs);
    snapshot = await read();
  }
  return snapshot;
}
