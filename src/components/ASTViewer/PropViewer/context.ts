import { createContext } from "react";
import type { Node, SourceFile } from "typescript";

export const SourceFileContext = createContext<SourceFile>(null!);
SourceFileContext.displayName = "SourceFileContext";

export interface PropViewerContext {
    onSelectNode(node: Node): void;
}

export const PropViewerContext = createContext<PropViewerContext>(null!);
PropViewerContext.displayName = "PropViewerContext";
