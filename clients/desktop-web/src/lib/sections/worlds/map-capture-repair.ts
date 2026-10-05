import type { Schema } from '../shared/types';

type Request = Schema['MapCaptureRequestDTO'];
type Report = Schema['MapAssetsReportDTO'];

export function sameBinding(
  a: Schema['MapAssetsBindingDTO'],
  b: Schema['MapAssetsBindingDTO'],
): boolean {
  return (
    a.agentHostId === b.agentHostId &&
    a.serverId === b.serverId &&
    a.slotId === b.slotId &&
    a.worldIncarnation === b.worldIncarnation &&
    a.revision === b.revision
  );
}
export function sameArea(a: Schema['MapAssetsAreaDTO'], b: Schema['MapAssetsAreaDTO']): boolean {
  return (
    a.min.every((value, i) => value === b.min[i]) && a.max.every((value, i) => value === b.max[i])
  );
}
export function sameCaptureRequest(a: Request, b: Request): boolean {
  return (
    sameBinding(a.binding, b.binding) &&
    sameArea(a.area, b.area) &&
    sameArea(a.contextArea, b.contextArea) &&
    a.dimension === b.dimension &&
    a.format === b.format &&
    a.minecraftVersion === b.minecraftVersion &&
    a.loader === b.loader &&
    a.loaderVersion === b.loaderVersion &&
    a.snapshotId === b.snapshotId &&
    a.inputFingerprint === b.inputFingerprint &&
    a.geometryGenerationId === b.geometryGenerationId &&
    a.resourceGenerationId === b.resourceGenerationId &&
    a.contextDataFingerprint === b.contextDataFingerprint
  );
}
/** Scope reports hash their requested area; capture requests hash the larger neighbor context. */
export function matchingCaptureReport(
  after: Report,
  before: Report,
  request: Request,
  operationId: string,
): boolean {
  return (
    after.operationId === operationId &&
    !!after.geometryGenerationId &&
    before.geometryGenerationId === request.geometryGenerationId &&
    before.resourceGenerationId === request.resourceGenerationId &&
    sameBinding(before.binding, request.binding) &&
    sameBinding(after.binding, request.binding) &&
    before.dimension === request.dimension &&
    after.dimension === request.dimension &&
    sameArea(before.area, request.area) &&
    sameArea(after.area, request.area) &&
    after.snapshotId === before.snapshotId &&
    after.resourceGenerationId === request.resourceGenerationId
  );
}
export function confirmedCaptureRepair(
  after: Report,
  before: Report,
  request: Request,
  operationId: string,
): boolean {
  return (
    matchingCaptureReport(after, before, request, operationId) &&
    after.outcome === 'repaired' &&
    (after.counts.captured_appearance ?? 0) > 0 &&
    after.repair?.sameSavedArea === true &&
    after.repair.beforeOperationId === before.operationId
  );
}

/** Specific next actions for refusals; retain the original code for exported evidence. */
export function captureRemedy(detail: string): string | undefined {
  if (/java_version|java_probe|invalid_java_path/.test(detail))
    return 'Choose the Java executable for the previewed major version (17 or 21). Preparation probes it without launching Minecraft.';
  if (/configuration_receipt_required/.test(detail))
    return 'Inspect and apply the matching Prism instance again to record its configuration. Then preview a fresh saved capture.';
  if (
    /configuration_|source_closure|sources_changed|components_changed|resource_context|manifest_context|pack_order|pack_selection|client_pack_selection|selected_pack_order|selected_resource_order/.test(
      detail,
    )
  )
    return 'Reinspect and apply the exact client instance with its actual enabled packs and mod priority. Keep those files unchanged while preparing; then preview fresh context.';
  if (/client_version|vanilla_resource|invalid_minecraft_client/.test(detail))
    return 'Choose the client release and Minecraft client JAR for the exact game/loader version shown in the preview. A server JAR cannot supply client textures.';
  if (
    /snapshot_|saved_data_|context_.*mismatch|inputs_changed|stale_map_scene|capture base changed|binding_changed|expired/i.test(
      detail,
    )
  )
    return 'Discard the stale private preparation, refresh terrain after saving the world, and preview fresh context for the same affected blocks.';
  if (
    /saved_context_missing|missing_saved_chunk|unloaded|lighting_not_ready|lighting_timeout|light.*pending/i.test(
      detail,
    )
  )
    return 'Generate and save missing chunks in the original world, then refresh terrain and prepare fresh context. In the private client, load the requested chunks and wait for lighting before preparing/exporting.';
  if (
    /alternate.*renderer|renderer.*namespace|unsupported.*shader|unsupported.*material/i.test(
      detail,
    )
  )
    return 'If a client-only shader or alternate renderer causes this refusal, use a separate matching client instance with default rendering, import its resources and recapture. A block that still needs unsupported material behavior requires an adapter change; export the report and refusal.';
  if (
    /saved_block_registry_missing|saved_block_property_missing|block_entity_renderer_missing|saved_dimension_missing/.test(
      detail,
    )
  )
    return 'The selected client cannot reproduce this saved block or dimension. Import the complete matching client mod set and prepare fresh context. If its renderer is still absent, export the report and refusal for an adapter correction.';
  if (/byte_limit|entry_limit|input_budget|256 MiB|capture.*limit/i.test(detail))
    return 'Preview a smaller saved area around the affected blocks. A client input-budget failure also requires a smaller matching instance; do not remove required server mods.';
  if (
    /custom_client|matching_capture_helper|unsupported_capture_loader|helpers_unavailable/.test(
      detail,
    )
  )
    return 'Use an MSC release containing an adapter for this exact loader/client configuration. Nearby versions are unsafe substitutes; retain the existing map and export its report if no exact adapter exists.';
  return undefined;
}
