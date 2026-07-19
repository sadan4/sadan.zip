import { ModuleViewerStore, useModuleViewerSettingsStore, useModuleViewerStore } from "@/routes/e/-data";
import type { TModuleId } from "@/utils/types";
import type { Edge, Node } from "@xyflow/react";

import { useEffect, useState } from "react";

export interface GeneratedGraph {
    nodes: Node[];
    edges: Edge[];
}

export function useModuleGraph2() {
    const moduleId = useModuleViewerStore(({ selectedModule }) => selectedModule);
    const graphDepth = useModuleViewerSettingsStore(({ graphDepth }) => graphDepth);
    const [graph, setGraph] = useState<GeneratedGraph | null>(null);

    useEffect(() => {
        let cancelled = false;

        void async function () {
            if (!moduleId)
                return;

            const { _buildService: buildService } = ModuleViewerStore.getState();
            const graph = await buildService.generateModuleGraph(moduleId as TModuleId, graphDepth);

            // oxlint-disable-next-line typescript/no-unnecessary-condition -- ts bug?
            if (cancelled) {
                // ^?
                return;
            }

            setGraph(graph);
        }();

        return () => {
            cancelled = true;
        };
    }, [moduleId, graphDepth]);
    return graph;
}
