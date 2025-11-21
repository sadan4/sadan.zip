import { Boilerplate } from "@/components/Boilerplate";
import { MonacoCodeEditor, type MonacoCodeEditorHandle } from "@/components/CodeEditor/Monaco";
import { ResizableSidebar, Side, SidebarStateStoreProvider } from "@/components/layout/ResizableSidebar";
import { useConsoleHelpers } from "@/hooks/consoleHelpers";
import { useSourceFile } from "@/hooks/sourceFile";
import cn from "@/utils/cn";
import { TreeMode } from "@/utils/typescript";

import { NodeTree } from "./NodeTree";
import { astViewerStore, leftAstSidebarStateStore, rightAstSidebarStateStore, updateASTViewerCode } from "./store";
import styles from "./styles.module.css";

import * as monaco from "monaco-editor";
import { useRef, useState } from "react";
import ts, { SyntaxKind } from "typescript";


export default function ASTViewer() {
    const { code, language, theme } = astViewerStore.useShallow(({ code, language, theme }) => ({
        code,
        language,
        theme,
    }));

    const sidebarBoundingRef = useRef<HTMLDivElement>(null);
    const editorRef = useRef<MonacoCodeEditorHandle>(null);
    const [sourceFile, { reparseCount }] = useSourceFile(code, language);
    const [selectedNode, setSelectedNode] = useState<ts.Node | undefined>(undefined);

    function rangeFromNode(node: ts.Node): monaco.Range {
        const start = sourceFile.getLineAndCharacterOfPosition(node.pos);
        const end = sourceFile.getLineAndCharacterOfPosition(node.end);

        return new monaco.Range(
            start.line + 1,
            start.character + 1,
            end.line + 1,
            end.character + 1,
        );
    }

    const editorHighlights = selectedNode ? [rangeFromNode(selectedNode)] : [];

    useConsoleHelpers({
        sourceFile,
        ts,
        node: selectedNode,
    });

    function onSelectNode(node: ts.Node) {
        setSelectedNode(node);
    }

    return (
        <>
            <Boilerplate />
            <div className={cn(styles.container)}>
                <header className={styles.header}>header</header>
                <div
                    className={styles.main}
                    ref={sidebarBoundingRef}
                >
                    <SidebarStateStoreProvider store={leftAstSidebarStateStore}>
                        <ResizableSidebar
                            boundingElement={sidebarBoundingRef}
                            side={Side.LEFT}
                            defaultSize={1 / 3}
                        >
                            <MonacoCodeEditor
                                ref={editorRef}
                                language={language}
                                theme={theme}
                                code={code}
                                onChange={(newCode) => {
                                    updateASTViewerCode(newCode);
                                }}
                                highlights={editorHighlights}
                            />
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                    <div className="h-full shrink grow">
                        <NodeTree
                            root={sourceFile}
                            treeMode={TreeMode.GET_CHILDREN}
                            reparseCount={reparseCount}
                            onSelectNode={onSelectNode}
                            highlightedNodes={selectedNode && [selectedNode]}
                            selectedNode={selectedNode}
                        />
                    </div>
                    <SidebarStateStoreProvider store={rightAstSidebarStateStore}>
                        <ResizableSidebar
                            boundingElement={sidebarBoundingRef}
                            side={Side.RIGHT}
                            defaultSize={1 - (1 / 3)}
                        >
                            Selected Node: {SyntaxKind[selectedNode?.kind ?? SyntaxKind.Unknown]}
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                </div>
            </div>
        </>
    );
}
