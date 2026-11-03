import type { AgentServiceStatus } from '../../platform';

/** One native query at a time. A timeout changes the display without spawning
 * more native workers while an earlier OS service query is still pending. */
export function createServiceStatusMonitor(
  read: () => Promise<AgentServiceStatus>,
  update: (status: AgentServiceStatus) => void,
  failed: (error: unknown) => void,
) {
  let context: string | undefined;
  let generation = 0;
  let pending = false;
  let disposed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let deadline: ReturnType<typeof setTimeout> | undefined;

  function waitForStatus(): void {
    clearTimeout(deadline);
    const request = generation;
    deadline = setTimeout(() => {
      if (!disposed && context && request === generation)
        failed(
          new Error('The local agent service status check did not respond within 10 seconds.'),
        );
    }, 10_000);
  }

  async function refresh(): Promise<void> {
    if (!context || pending || disposed) return;
    clearTimeout(timer);
    const request = generation;
    pending = true;
    const current = () => !disposed && !!context && request === generation;
    waitForStatus();
    try {
      const status = await read();
      if (current()) update(status);
    } catch (error) {
      if (current()) failed(error);
    } finally {
      if (request === generation) clearTimeout(deadline);
      pending = false;
      if (!disposed && context) {
        if (request !== generation) void refresh();
        else timer = setTimeout(() => void refresh(), 5000);
      }
    }
  }

  return {
    configure(next: string | undefined): void {
      if (disposed || context === next) return;
      context = next;
      generation += 1;
      clearTimeout(timer);
      clearTimeout(deadline);
      if (context) {
        // Reopening while an older native query is stuck still needs a
        // bounded display state, even though it cannot start another query.
        waitForStatus();
        void refresh();
      }
    },
    refresh,
    dispose(): void {
      disposed = true;
      generation += 1;
      clearTimeout(timer);
      clearTimeout(deadline);
    },
  };
}
