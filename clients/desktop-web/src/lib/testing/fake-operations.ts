import type { components } from '../api/generated';

/** Provides stable operation snapshots for HTTP polling and WebSocket reconnect tests. */
export class FakeOperationStore {
  private readonly operations = new Map<string, components['schemas']['OperationDTO']>();

  add(operation: components['schemas']['OperationDTO']): void {
    this.operations.set(operation.id, structuredClone(operation));
  }

  get(id: string): components['schemas']['OperationDTO'] | undefined {
    const operation = this.operations.get(id);
    return operation ? structuredClone(operation) : undefined;
  }

  update(
    id: string,
    patch: Partial<components['schemas']['OperationDTO']>,
  ): components['schemas']['OperationDTO'] {
    const operation = this.operations.get(id);
    if (!operation) {
      throw new Error(`unknown operation: ${id}`);
    }
    const updated = { ...operation, ...structuredClone(patch) };
    this.operations.set(id, updated);
    return structuredClone(updated);
  }
}
