import { cn } from "@/utils/cn";

import { defaultInitialSize, HIDE_THRESHOLD, Side, SidebarStateStoreContext, useSidebarStateStore } from "./store";
import { type ResizeHandleAPI, VerticalResizeHandle } from "../ResizeHandle";

import { type PropsWithChildren, type RefObject, useContext, useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";

export interface SidebarProps extends PropsWithChildren {
    side: Side;
    boundingElement: RefObject<HTMLDivElement | null>;
    defaultSize?: number;
}

export function ResizableSidebar({
    side,
    defaultSize = defaultInitialSize(side),
    children,
    boundingElement,
}: SidebarProps) {
    const store = useContext(SidebarStateStoreContext)!;
    const [contentRef, setContentRef] = useState<HTMLDivElement | null>(null);
    const sidebarApiRef = useRef<ResizeHandleAPI | null>(null);

    const { hidden, handleHidden } = useSidebarStateStore(useShallow(({ hidden, handleHidden }) => ({
        hidden,
        handleHidden,
    })));

    const [shouldDispatch, setShouldDispatch] = useState(true);

    useEffect(() => {
        store.getState().setRef(contentRef);
        () => {
            store.getState().setRef(null);
        };
    }, [contentRef, store]);

    useEffect(() => {
        store.getState().setSidebarApi({
            get current() {
                if (!sidebarApiRef.current) {
                    return null;
                }
                return {
                    reset() {
                        sidebarApiRef.current?.reset();
                    },
                    setCurrentPos(percent, dispatchResize) {
                        if (side === Side.RIGHT) {
                            percent = 100 - percent;
                        }
                        sidebarApiRef.current?.setCurrentPos(percent, dispatchResize);
                    },
                } satisfies ResizeHandleAPI;
            },
        });
        return () => {
            store.getState().setSidebarApi();
        };
    }, [side, sidebarApiRef, store]);

    return (
        <>
            {side === Side.LEFT && (
                <div
                    ref={setContentRef}
                    className={cn(hidden && "hidden", "overflow-x-hidden")}
                >
                    {children}
                </div>
            )}
            <VerticalResizeHandle
                boundingElementRef={boundingElement}
                ref={sidebarApiRef}
                initialPosition={defaultSize}
                onResize={(pos) => {
                    if (side === Side.RIGHT) {
                        pos = 100 - pos;
                    }
                    if (shouldDispatch) {
                        store.getState().setWidth(pos);
                    }
                }}
                onResizeFinish={() => {
                    // we have gone past the min-width, but haven't passed (min-width / 2)
                    if (handleHidden) {
                        store.getState().sidebarApi.current?.setCurrentPos(HIDE_THRESHOLD);
                        store.getState().setWidth(HIDE_THRESHOLD);
                    }
                    setShouldDispatch(!hidden);
                }}
                className={cn((hidden || handleHidden) && "pointer-events-none opacity-0", shouldDispatch || "pointer-events-none")}
            />
            {side === Side.RIGHT && (
                <div
                    ref={setContentRef}
                    className={cn(hidden && "hidden", "overflow-x-hidden")}
                >
                    {children}
                </div>
            )}
        </>
    );
}
