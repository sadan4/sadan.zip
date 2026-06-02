import { ModuleViewerStore, useModuleViewerStore } from "@/routes/e/-data";
import type { TModuleId } from "@/utils/types";
import type { Edge, Node } from "@xyflow/react";

import { useEffect, useState } from "react";

export interface GeneratedGraph {
    nodes: Node[];
    edges: Edge[];
}

export function useModuleGraph2() {
    const moduleId = useModuleViewerStore(({ selectedModule }) => selectedModule);
    const [graph, setGraph] = useState<GeneratedGraph | null>(null);

    useEffect(() => {
        !async function () {
            if (!moduleId)
                return;

            const { _buildService: buildService } = ModuleViewerStore.getState();
            const graph = await buildService.generateModuleGraph(moduleId as TModuleId);

            setGraph(graph);
        }();
    }, [moduleId]);
    return graph;
}
