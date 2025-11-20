import { Boilerplate } from "@/components/Boilerplate";
import { MonacoCodeEditor, type MonacoCodeEditorHandle } from "@/components/CodeEditor/Monaco";
import { ResizableSidebar, Side, SidebarStateStoreProvider } from "@/components/layout/ResizableSidebar";
import { useConsoleHelpers } from "@/hooks/consoleHelpers";
import { useSourceFile } from "@/hooks/sourceFile";
import { TreeMode } from "@/utils/typescript";

import { NodeTree } from "./NodeTree";
import { leftAstSidebarStateStore, rightAstSidebarStateStore, updateASTViewerCode, useASTViewerStore } from "./store";

import { useRef, useState } from "react";
import ts from "typescript";


export default function ASTViewer() {
    const { code, language, theme } = useASTViewerStore.useShallow(({ code, language, theme }) => ({
        code,
        language,
        theme,
    }));

    const sidebarBoundingRef = useRef<HTMLElement>(null);
    const editorRef = useRef<MonacoCodeEditorHandle>(null);
    const [sourceFile, { reparseCount }] = useSourceFile(code, language);
    const [selectedNode, setSelectedNode] = useState<ts.Node | undefined>(undefined);

    useConsoleHelpers({
        sourceFile,
        ts,
    });

    function onSelectNode(node: ts.Node) {
        setSelectedNode(node);
    }

    return (
        <>
            <Boilerplate />
            <div className="flex h-full w-full flex-col">
                <header>header</header>
                <main
                    className="flex grow"
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
                            />
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                    <div className="grow">
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
                            right sidebar
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                </main>
            </div>
        </>
    );
}
