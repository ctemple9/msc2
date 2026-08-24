import type { components } from '../api/generated';

export type OperationState = 'idle' | 'unsupported' | 'recovering' | 'ready';

/** Holds only current operation snapshots; reconnects replace, rather than replay, them. */
export class OperationTracker {
  private readonly values = new Map<string, components['schemas']['OperationDTO']>();
  private stateValue: OperationState = 'idle';

  get state(): OperationState {
    return this.stateValue;
  }

  setUnsupported(): void {
    this.stateValue = 'unsupported';
  }

  beginRecovery(): void {
    this.stateValue = 'recovering';
  }

  completeRecovery(): void {
    this.stateValue = 'ready';
  }

  upsert(operation: components['schemas']['OperationDTO']): void {
    this.values.set(operation.id, structuredClone(operation));
    this.stateValue = 'ready';
  }

  get(id: string): components['schemas']['OperationDTO'] | undefined {
    const operation = this.values.get(id);
    return operation ? structuredClone(operation) : undefined;
  }

  all(): readonly components['schemas']['OperationDTO'][] {
    return structuredClone([...this.values.values()]);
  }

  removeTerminal(): void {
    for (const [id, operation] of this.values) {
      if (isTerminal(operation)) this.values.delete(id);
    }
  }
}

export function isTerminal(operation: components['schemas']['OperationDTO']): boolean {
  return ['succeeded', 'failed', 'cancelled'].includes(operation.state);
}
