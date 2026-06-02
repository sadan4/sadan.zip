// import { ModuleViewerStore, useModuleViewerStore } from "@/routes/e/-data";
// import cn from "@/utils/cn";
// import { entries } from "@/utils/obj";
// import type { Edge as GraphEdge, Node as GraphNode } from "@xyflow/react";

// import { type DepsJson, MainDepsEntryValue, TModuleId } from "../../server/types";
// import styles from "../routes/e/-styles.module.scss";

// import ELK, { type ElkNode } from "elkjs";
// import { useEffect, useState } from "react";

// interface OurNode extends ElkNode {
//     data: {
//         label: string;
//     };
// }

// export function useModuleGraph(depth = 1): GeneratedGraph | null {
//     const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
//     const selectedModuleId = useModuleViewerStore(({ selectedModule }) => selectedModule);
//     const [depGraph, setDepGraph] = useState<DepsJson | null>(null);
//     const [laidOutGraph, setLaidOutGraph] = useState<GeneratedGraph | null>(null);

//     useEffect(() => {
//         if (!buildHash) {
//             return;
//         }
//         !async function () {
//             const depGraph = await ModuleViewerStore.getState().getDepsGraph();

//             setDepGraph(depGraph);
//         }();
//     }, [buildHash]);

//     useEffect(() => {
//         let depMap: Map<TModuleId, MainDepsEntryValue> | null = null;
//         let reverseDepMap: Map<TModuleId, TModuleId[]> | null = null;

//         if (depGraph) {
//             depMap = new Map(entries(depGraph.deps));
//             reverseDepMap = new Map();
//             for (const [to, from] of depMap) {
//                 for (const fromId of from.syncUses) {
//                     if (!reverseDepMap.has(fromId)) {
//                         reverseDepMap.set(fromId, []);
//                     }
//                     reverseDepMap.get(fromId)!.push(to);
//                 }
//             }
//         }

//         !async function () {
//             const graph = await layoutGraph({
//                 depMap,
//                 reverseDepMap,
//                 selectedModuleId,
//                 depth,
//             });

//             setLaidOutGraph(graph);
//         }();
//     }, [depGraph, depth, selectedModuleId]);

//     return laidOutGraph;
// }

// interface LayoutGraphOpts {
//     depMap: Map<TModuleId, { syncUses: TModuleId[]; }> | null;
//     reverseDepMap: Map<TModuleId, TModuleId[]> | null;
//     selectedModuleId: TModuleId | null;
//     depth: number;
// }

// async function layoutGraph({
//     depMap,
//     reverseDepMap,
//     selectedModuleId,
//     depth,
// }: LayoutGraphOpts): Promise<GeneratedGraph | null> {
//     if (!depMap || !reverseDepMap || !selectedModuleId) {
//         return null;
//     }

//     let curDepth = depth;
//     let queue = [selectedModuleId];
//     const includedNodes = new Set<TModuleId>([selectedModuleId]);

//     // probe down
//     while (curDepth-- > 0) {
//         const newQueue: TModuleId[] = [];

//         for (const moduleId of queue) {
//             const outgoing = reverseDepMap.get(moduleId) || [];

//             for (const id of outgoing) {
//                 includedNodes.add(id);
//                 newQueue.push(id);
//             }
//         }
//         queue = newQueue;
//     }

//     // probe up
//     // reset depth
//     curDepth = depth;
//     // reset queue
//     queue = [selectedModuleId];
//     while (curDepth-- > 0) {
//         const newQueue: TModuleId[] = [];

//         for (const moduleId of queue) {
//             const incoming = depMap.get(moduleId)?.syncUses ?? [];

//             for (const id of incoming) {
//                 includedNodes.add(id);
//                 newQueue.push(id);
//             }
//         }
//         queue = newQueue;
//     }

//     const edges: [TModuleId, TModuleId][] = [...depMap.entries()].flatMap(([moduleId, { syncUses }]) => {
//         if (!includedNodes.has(moduleId)) {
//             return [];
//         }

//         const ret: [TModuleId, TModuleId][] = [];

//         for (const use of syncUses) {
//             if (!includedNodes.has(use)) {
//                 continue;
//             }

//             ret.push([moduleId, use]);
//         }

//         return ret;
//     });

//     const elkRootNode = {
//         id: "root",
//         layoutOptions: {
//             "elk.direction": "DOWN",
//         },
//         children: Array.from(includedNodes).map((id) => {
//             return {
//                 id,
//                 width: 75,
//                 height: 40,
//                 data: {
//                     label: id,
//                 },
//             } satisfies OurNode;
//         }),
//         edges: edges.map(([from, to]) => {
//             return {
//                 id: `${from}->${to}`,
//                 sources: [from],
//                 targets: [to],
//             };
//         }),
//     } satisfies ElkNode;

//     const layoutResult = await new ELK().layout(elkRootNode);

//     const nodes = layoutResult.children!.map((child) => {
//         return {
//             id: child.id,
//             position: {
//                 x: child.x!,
//                 y: child.y!,
//             },
//             data: {
//                 label: child.data.label,
//             },
//             width: child.width!,
//             height: child.height!,
//             className: cn(child.id === selectedModuleId && styles.activeNode),
//             draggable: true,
//         } satisfies GraphNode;
//     });

//     return {
//         nodes,
//         edges: edges.map(([from, to]): GraphEdge => {
//             return {
//                 id: `${from}->${to}`,
//                 source: from,
//                 target: to,
//             };
//         }),
//     };
// }
