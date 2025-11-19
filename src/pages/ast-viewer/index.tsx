import { Boilerplate } from "@/components/Boilerplate";
import { MonacoCodeEditor, type MonacoCodeEditorHandle } from "@/components/CodeEditor/Monaco";
import { ResizableSidebar, Side, SidebarStateStoreProvider } from "@/components/layout/ResizableSidebar";

import { leftAstSidebarStateStore, rightAstSidebarStateStore, updateASTViewerCode, useASTViewerStore } from "./store";

import { useRef } from "react";


export default function ASTViewer() {
    const sidebarBoundingRef = useRef<HTMLElement>(null);
    const editorRef = useRef<MonacoCodeEditorHandle>(null);

    const { code, language, theme } = useASTViewerStore.useShallow(({ code, language, theme }) => ({
        code,
        language,
        theme,
    }));

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
                                initialCode={code}
                                onChange={(newCode) => {
                                    updateASTViewerCode(newCode);
                                }}
                            />
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                    <div className="grow bg-secondary-500/50 p-3">
                        <div className="flex w-fit flex-col gap-3">
                            main body
                        </div>
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
