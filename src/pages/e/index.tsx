import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { createSidebarStateStore, ResizableSidebar, Side, SidebarStateStoreProvider } from "@/components/layout/ResizableSidebar";

import { useRef } from "react";
import { useStore } from "zustand";

export const leftSidebarStateStore = createSidebarStateStore();
export const rightSidebarStateStore = createSidebarStateStore();

export default function Explorer() {
    const sidebarBoundingRef = useRef<HTMLDivElement>(null);
    const rightSidebarHidden = useStore(rightSidebarStateStore, ({ hidden }) => hidden);
    const leftSidebarHidden = useStore(leftSidebarStateStore, ({ hidden }) => hidden);

    return (
        <>
            <Boilerplate />
            <div className="flex h-full flex-col">
                <div className="bg-primary-400/50 h-1/20">header</div>
                <div
                    className="relative flex grow"
                    ref={sidebarBoundingRef}
                >
                    <SidebarStateStoreProvider store={leftSidebarStateStore}>
                        <ResizableSidebar
                            boundingElement={sidebarBoundingRef}
                            side={Side.LEFT}
                        >
                            left sidebar
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                    <div className="bg-secondary-500/50 grow p-3">
                        <div className="flex w-fit flex-col gap-3">
                            main body
                            <Button onClick={() => {
                                leftSidebarHidden
                                    ? leftSidebarStateStore.getState().show()
                                    : leftSidebarStateStore.getState().hide();
                            }}
                            >
                                {leftSidebarHidden ? "Show" : "Hide"} Left Sidebar
                            </Button>
                            <Button onClick={() => {
                                rightSidebarHidden
                                    ? rightSidebarStateStore.getState().show()
                                    : rightSidebarStateStore.getState().hide();
                            }}
                            >
                                {rightSidebarHidden ? "Show" : "Hide"} Right Sidebar
                            </Button>
                        </div>
                    </div>
                    <SidebarStateStoreProvider store={rightSidebarStateStore}>
                        <ResizableSidebar
                            boundingElement={sidebarBoundingRef}
                            side={Side.RIGHT}
                        >
                            right sidebar
                        </ResizableSidebar>
                    </SidebarStateStoreProvider>
                </div>
            </div>
        </>
    );
}
