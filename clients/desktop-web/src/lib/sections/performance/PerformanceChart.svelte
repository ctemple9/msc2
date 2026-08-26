<script lang="ts">
  // A small history line/area chart for TPS-Over-Time and Player Activity.
  // MSC 1's Charts-framework equivalents (DetailsPerformanceCharts.swift)
  // are static -- no hover layer -- but the dataviz skill treats a
  // crosshair+tooltip as the non-negotiable default for any line/area
  // chart, and the step calls for the dataviz discipline explicitly, so
  // this adds one. Rainbow line gradients and area gradients in the oracle
  // are flattened to a single flat color per antiAIslop rule #2/#5 (no
  // decorative gradients) -- color is either the metric's live status tone
  // (TPS/CPU: an explicitly allowed "live-stat fill") or, for a series with
  // no defined state (Player Activity), a quiet neutral tone that spends
  // none of the 10% accent budget.
  import Icon from '../../components/base/Icon.svelte';

  export let samples: readonly number[] = [];
  export let domainMax: number;
  export let color: string;
  export let referenceValue: number | undefined = undefined;
  export let valueLabel: (value: number) => string = (value) => `${Math.round(value)}`;
  export let sampleIntervalMs: number;
  export let emptyIcon: 'waveform' | 'people';
  export let emptyMessage: string;

  const width = 300;
  const height = 130;
  const padLeft = 30;
  const padRight = 8;
  const padTop = 10;
  const padBottom = 8;
  const plotWidth = width - padLeft - padRight;
  const plotHeight = height - padTop - padBottom;

  $: ticks = [0, 0.5, 1].map((fraction) => domainMax * fraction);

  function xFor(index: number, count: number): number {
    if (count <= 1) return padLeft;
    return padLeft + (index / (count - 1)) * plotWidth;
  }
  function yFor(value: number): number {
    const clamped = Math.min(Math.max(value, 0), domainMax);
    return padTop + (1 - clamped / domainMax) * plotHeight;
  }

  $: points = samples.map((value, index) => ({
    x: xFor(index, samples.length),
    y: yFor(value),
    value,
  }));
  $: linePath = points
    .map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x.toFixed(1)},${p.y.toFixed(1)}`)
    .join(' ');
  $: areaPath =
    points.length > 0
      ? `${linePath} L${points[points.length - 1].x.toFixed(1)},${padTop + plotHeight} L${points[0].x.toFixed(1)},${padTop + plotHeight} Z`
      : '';
  $: refY = referenceValue !== undefined ? yFor(referenceValue) : undefined;

  let hoverIndex: number | undefined;

  function onMove(event: PointerEvent): void {
    if (points.length === 0) return;
    const svg = event.currentTarget as SVGSVGElement;
    const rect = svg.getBoundingClientRect();
    const relativeX = ((event.clientX - rect.left) / rect.width) * width;
    let nearest = 0;
    let nearestDistance = Infinity;
    points.forEach((point, index) => {
      const distance = Math.abs(point.x - relativeX);
      if (distance < nearestDistance) {
        nearestDistance = distance;
        nearest = index;
      }
    });
    hoverIndex = nearest;
  }
  function onLeave(): void {
    hoverIndex = undefined;
  }

  $: hoverPoint = hoverIndex !== undefined ? points[hoverIndex] : undefined;
  $: hoverAgoSeconds =
    hoverIndex !== undefined
      ? Math.round(((samples.length - 1 - hoverIndex) * sampleIntervalMs) / 1000)
      : 0;
  $: tooltipX = hoverPoint
    ? Math.min(Math.max(hoverPoint.x, padLeft + 26), width - padRight - 26)
    : 0;
</script>

{#if samples.length < 2}
  <div class="empty" style="min-height: {height}px;">
    <Icon name={emptyIcon} size={22} />
    <p>{emptyMessage}</p>
  </div>
{:else}
  <svg
    viewBox="0 0 {width} {height}"
    class="chart"
    role="img"
    aria-label="History chart, latest value {valueLabel(samples[samples.length - 1])}"
    onpointermove={onMove}
    onpointerleave={onLeave}
  >
    {#each ticks as tick (tick)}
      <line x1={padLeft} x2={width - padRight} y1={yFor(tick)} y2={yFor(tick)} class="grid-line" />
      <text x={padLeft - 6} y={yFor(tick) + 3} class="tick-label" text-anchor="end"
        >{valueLabel(tick)}</text
      >
    {/each}

    {#if refY !== undefined}
      <line x1={padLeft} x2={width - padRight} y1={refY} y2={refY} class="reference-line" />
    {/if}

    <path d={areaPath} fill={color} fill-opacity="0.14" stroke="none" />
    <path d={linePath} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" />

    {#if points.length > 0}
      <circle
        cx={points[points.length - 1].x}
        cy={points[points.length - 1].y}
        r="3"
        fill={color}
      />
    {/if}

    {#if hoverPoint}
      <line
        x1={hoverPoint.x}
        x2={hoverPoint.x}
        y1={padTop}
        y2={padTop + plotHeight}
        class="crosshair"
      />
      <circle cx={hoverPoint.x} cy={hoverPoint.y} r="3.5" fill={color} class="hover-dot" />
      <g transform="translate({tooltipX}, {Math.max(hoverPoint.y - 26, padTop)})">
        <rect x="-26" y="-14" width="52" height="20" rx="5" class="tooltip-bg" />
        <text x="0" y="0" text-anchor="middle" class="tooltip-text"
          >{valueLabel(hoverPoint.value)}</text
        >
      </g>
      <text x={hoverPoint.x} y={height - 2} text-anchor="middle" class="tick-label"
        >{hoverAgoSeconds === 0 ? 'now' : `-${hoverAgoSeconds}s`}</text
      >
    {/if}
  </svg>
{/if}

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--msc2-text-tertiary);
  }
  .empty p {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    text-align: center;
  }
  .chart {
    display: block;
    width: 100%;
    height: auto;
    cursor: crosshair;
  }
  .grid-line {
    stroke: var(--msc2-hairline-faint);
    stroke-width: 1;
  }
  .reference-line {
    stroke: var(--msc2-hairline);
    stroke-width: 1;
    stroke-dasharray: 3 3;
  }
  .tick-label {
    font-size: 8px;
    fill: var(--msc2-text-tertiary);
  }
  .crosshair {
    stroke: var(--msc2-hairline);
    stroke-width: 1;
  }
  .hover-dot {
    stroke: var(--msc2-tier-content);
    stroke-width: 1.5;
  }
  .tooltip-bg {
    fill: var(--msc2-neutral-elevated);
  }
  .tooltip-text {
    font-size: 10px;
    font-weight: 500;
    fill: var(--msc2-text-primary);
  }
</style>
