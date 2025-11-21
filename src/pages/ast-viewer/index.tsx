import { Boilerplate } from "@/components/Boilerplate";
import { MonacoCodeEditor, type MonacoCodeEditorHandle } from "@/components/CodeEditor/Monaco";
import { ResizableSidebar, Side, SidebarStateStoreProvider } from "@/components/layout/ResizableSidebar";
import { useConsoleHelpers } from "@/hooks/consoleHelpers";
import { useSourceFile } from "@/hooks/sourceFile";
import cn from "@/utils/cn";
import { getVisibleNodeRange, TreeMode } from "@/utils/typescript";

import { NodeTree } from "./NodeTree";
import { astViewerStore, leftAstSidebarStateStore, rightAstSidebarStateStore, updateASTViewerCode } from "./store";
import styles from "./styles.module.scss";

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
        const [pos, end] = getVisibleNodeRange(node, sourceFile);
        const startLineAndCharacter = sourceFile.getLineAndCharacterOfPosition(pos);
        const endLineAndCharacter = sourceFile.getLineAndCharacterOfPosition(end);

        return new monaco.Range(
            startLineAndCharacter.line + 1,
            startLineAndCharacter.character + 1,
            endLineAndCharacter.line + 1,
            endLineAndCharacter.character + 1,
        );
    }

    const editorHighlights = selectedNode ? [rangeFromNode(selectedNode)] : [];

    useConsoleHelpers({
        sourceFile,
        ts,
        node: selectedNode,
    });

    function onSelectNode(node: ts.Node) {
        if (editorRef.current) {
            const range = rangeFromNode(node);

            editorRef.current.editor.revealRangeInCenterIfOutsideViewport(range, monaco.editor.ScrollType.Immediate);
        }
        setSelectedNode(node);
    }

    return (
        <>
            <Boilerplate />
            <div className={cn(styles.container)}>
                <header className={styles.header}>header</header>
                <div
                    className={cn(styles.main, "handle-hover-info-300/25 handle-transparent")}
                    ref={sidebarBoundingRef}
                >
                    <SidebarStateStoreProvider store={leftAstSidebarStateStore}>
                        <ResizableSidebar
                            boundingElement={sidebarBoundingRef}
                            side={Side.LEFT}
                            defaultSize={1 / 3}
                            handleClassName={styles.handle}
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
                            handleClassName={styles.handle}
                        >
                            Selected Node: {SyntaxKind[selectedNode?.kind ?? SyntaxKind.Unknown]}
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                </div>
            </div>
        </>
    );
}
