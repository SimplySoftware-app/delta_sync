"use client"

import { useSyncExternalStore } from "react"
import {
  Background,
  Handle,
  MarkerType,
  Position,
  ReactFlow,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react"
import { BadgeCheck, ScanSearch, ShieldCheck, type LucideIcon } from "lucide-react"

import "@xyflow/react/dist/style.css"

export type DiagramPillar = { title: string; short: string }

type PillarData = DiagramPillar & { index: number; vertical: boolean }
type PillarNode = Node<PillarData, "pillar">

const icons: LucideIcon[] = [BadgeCheck, ShieldCheck, ScanSearch]

const NODE_W = 224
const noop = () => () => {}

function subscribeNarrow(cb: () => void) {
  const mq = window.matchMedia("(max-width: 639px)")
  mq.addEventListener("change", cb)
  return () => mq.removeEventListener("change", cb)
}

function PillarNodeView({ data }: NodeProps<PillarNode>) {
  const Icon = icons[data.index] ?? BadgeCheck
  const target = data.vertical ? Position.Top : Position.Left
  const source = data.vertical ? Position.Bottom : Position.Right
  return (
    <div
      style={{ width: NODE_W }}
      className="flex items-center gap-3 rounded-[16px] border border-border bg-card px-4 py-3 text-card-foreground shadow-sm"
    >
      {data.index > 0 && (
        <Handle type="target" position={target} className="!opacity-0" />
      )}
      <span className="flex size-10 shrink-0 items-center justify-center rounded-[10px] border border-border bg-muted text-foreground">
        <Icon className="size-5" aria-hidden="true" />
      </span>
      <span className="flex min-w-0 flex-col">
        <span className="text-base leading-tight font-medium text-foreground">
          {data.title}
        </span>
        <span className="text-sm leading-snug text-muted-foreground">
          {data.short}
        </span>
      </span>
      {data.index < 2 && (
        <Handle type="source" position={source} className="!opacity-0" />
      )}
    </div>
  )
}

const nodeTypes = { pillar: PillarNodeView }

export function PillarsDiagram({
  pillars,
  edgeLabels,
}: {
  pillars: DiagramPillar[]
  edgeLabels: [string, string]
}) {
  const mounted = useSyncExternalStore(
    noop,
    () => true,
    () => false,
  )
  const vertical = useSyncExternalStore(
    subscribeNarrow,
    () => window.matchMedia("(max-width: 639px)").matches,
    () => false,
  )

  const gap = 130
  const nodes: PillarNode[] = pillars.map((p, i) => ({
    id: `p${i}`,
    type: "pillar",
    position: vertical ? { x: 0, y: i * (64 + gap) } : { x: i * (NODE_W + gap + 20), y: 0 },
    data: { ...p, index: i, vertical },
    draggable: false,
    selectable: false,
    connectable: false,
  }))

  const edges: Edge[] = [0, 1].map((i) => ({
    id: `e${i}`,
    source: `p${i}`,
    target: `p${i + 1}`,
    type: "smoothstep",
    animated: true,
    label: edgeLabels[i],
    style: { stroke: "var(--muted-foreground)", strokeWidth: 1.5 },
    markerEnd: { type: MarkerType.ArrowClosed, color: "var(--muted-foreground)" },
    labelStyle: { fill: "var(--foreground)", fontSize: 12, fontWeight: 500 },
    labelBgStyle: { fill: "var(--background)", stroke: "var(--border)" },
    labelBgPadding: [8, 4],
    labelBgBorderRadius: 10,
  }))

  return (
    <div
      className="h-[460px] w-full overflow-hidden rounded-[16px] border border-border bg-background sm:h-[200px]"
      aria-hidden="true"
    >
      {mounted && (
        <ReactFlow
          key={vertical ? "v" : "h"}
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          nodesDraggable={false}
          nodesConnectable={false}
          elementsSelectable={false}
          zoomOnScroll={false}
          zoomOnPinch={false}
          zoomOnDoubleClick={false}
          panOnDrag={false}
          panOnScroll={false}
          preventScrolling={false}
          fitView
          fitViewOptions={{ padding: 0.12 }}
          minZoom={0.2}
          maxZoom={1.2}
        >
          <Background gap={20} size={1} color="var(--border)" />
        </ReactFlow>
      )}
    </div>
  )
}
