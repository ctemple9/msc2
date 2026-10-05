import { describe, expect, it } from 'vitest';
import {
  confirmedCaptureRepair,
  matchingCaptureReport,
  sameCaptureRequest,
} from '../../src/lib/sections/worlds/map-capture-repair';
import type { Schema } from '../../src/lib/sections/shared/types';

const binding = {
  agentHostId: 'host-a',
  serverId: 'server-a',
  slotId: 'world-a',
  worldIncarnation: 'save-a',
  revision: 'revision-a',
};
const area = { min: [0, 64, 0], max: [1, 65, 1] };
const request: Schema['MapCaptureRequestDTO'] = {
  format: 'msc-contextual-mesh-1',
  binding,
  area,
  contextArea: { min: [-1, 63, -1], max: [2, 66, 2] },
  geometryGenerationId: 'before-geometry',
  resourceGenerationId: 'resources',
  inputFingerprint: 'inputs',
  snapshotId: 'neighbor-context-hash',
  contextDataFingerprint: 'saved-data',
  dimension: 'minecraft:overworld',
  minecraftVersion: '1.20.1',
  loader: 'forge',
  loaderVersion: '47.4.10',
};
function report(): Schema['MapAssetsReportDTO'] {
  return {
    schemaVersion: 1,
    binding,
    area,
    dimension: request.dimension,
    snapshotId: 'affected-area-hash',
    snapshotMinecraftVersion: '1.20.1',
    resourceGenerationId: 'resources',
    geometryGenerationId: 'before-geometry',
    counts: { missing_context: 1 },
    diagnostics: [],
    operationId: 'before-check',
    outcome: 'needs_input',
    distinctStates: 1,
    inspectedBlocks: 1,
    inspectedChunks: 1,
    omittedIssues: 0,
    omittedSamples: 0,
    sources: [],
    scope: 'saved blocks',
    visibleFaces: 1,
    visualAcceptance: 'pending',
  };
}
function repaired(): Schema['MapAssetsReportDTO'] {
  return {
    ...report(),
    operationId: 'capture-import',
    geometryGenerationId: 'after-geometry',
    counts: { captured_appearance: 1 },
    outcome: 'repaired',
    repair: {
      beforeOperationId: 'before-check',
      sourceGenerationId: 'resources',
      targetGenerationId: 'resources',
      sameSavedArea: true,
      beforeCounts: { missing_context: 1 },
      afterCounts: { captured_appearance: 1 },
    },
  };
}

describe('capture repair approval', () => {
  it('accepts a verified scoped comparison even though the neighbor-context hash differs', () => {
    expect(confirmedCaptureRepair(repaired(), report(), request, 'capture-import')).toBe(true);
    expect(sameCaptureRequest(request, structuredClone(request))).toBe(true);
  });
  it('rejects another host, world, snapshot, area or capture base', () => {
    const valid = repaired();
    for (const changed of [
      { ...valid, binding: { ...binding, agentHostId: 'host-b' } },
      { ...valid, binding: { ...binding, slotId: 'world-b' } },
      { ...valid, snapshotId: 'changed-save' },
      { ...valid, area: { ...area, max: [4, 65, 1] } },
    ])
      expect(matchingCaptureReport(changed, report(), request, 'capture-import')).toBe(false);
    expect(sameCaptureRequest(request, { ...request, geometryGenerationId: 'changed-base' })).toBe(
      false,
    );
  });
  it('keeps upload-only, unadopted, partial and unrelated-operation results from claiming repaired', () => {
    const valid = repaired();
    for (const changed of [
      { ...valid, geometryGenerationId: null },
      { ...valid, outcome: 'ready' as const },
      { ...valid, outcome: 'partially_repaired' as const },
      { ...valid, counts: { model_resolved: 1 } },
      { ...valid, operationId: 'other-operation' },
      { ...valid, repair: { ...valid.repair!, sameSavedArea: false } },
      { ...valid, repair: { ...valid.repair!, beforeOperationId: 'other-check' } },
    ])
      expect(confirmedCaptureRepair(changed, report(), request, 'capture-import')).toBe(false);
  });
});
