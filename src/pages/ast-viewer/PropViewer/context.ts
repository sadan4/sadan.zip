import { namedContext } from "@/utils/devtools";

import type { Node, SourceFile } from "typescript";

export const SourceFileContext = namedContext<SourceFile>(null!, "SourceFileContext");

export interface PropViewerContext {
    onSelectNode(node: Node): void;
}

export const PropViewerContext = namedContext<PropViewerContext>(null!, "PropViewerContext");
