import { useRectFromRef } from "@/hooks/rect";
import { cn } from "@/utils/cn";

import { defaultInitialSize, HIDE_THRESHOLD, Side, SidebarStateStoreContext, useSidebarStateStore } from "./store";
import { type ResizeHandleAPI, VerticalResizeHandle } from "../ResizeHandle";

import { type PropsWithChildren, type RefObject, useContext, useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";

export interface SidebarProps extends PropsWithChildren {
    side: Side;
    boundingElement: RefObject<HTMLElement | null>;
    defaultSize?: number;
    handleClassName?: string;
}

export function ResizableSidebar({
    side,
    defaultSize = defaultInitialSize(side),
    children,
    boundingElement,
    handleClassName,
}: SidebarProps) {
    const store = useContext(SidebarStateStoreContext)!;
    const [contentRef, setContentRef] = useState<HTMLDivElement | null>(null);
    const sidebarApiRef = useRef<ResizeHandleAPI | null>(null);

    const { hidden, handleHidden } = useSidebarStateStore(useShallow(({ hidden, handleHidden }) => ({
        hidden,
        handleHidden,
    })));

    const { top, height, width } = useRectFromRef(boundingElement) ?? {};
    const [shouldDispatch, setShouldDispatch] = useState(true);

    useEffect(() => {
        if (width != null) {
            store.getState().setContainerWidth(width);
        }
    }, [store, width]);

    useEffect(() => {
        store.getState().setRef(contentRef);
        () => {
            store.getState().setRef(null);
        };
    }, [contentRef, store]);

    useEffect(() => {
        store.getState().setSidebarApi({
            // FIXME: this is cursed as all hell
            // also makes react compiler unhappy
            // eslint-disable-next-line react-hooks/todo
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
                    className={cn(hidden && "hidden")}
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
                className={cn((hidden || handleHidden) && "pointer-events-none opacity-0", shouldDispatch || "pointer-events-none", handleClassName)}
                style={{
                    top,
                    height,
                }}
                minPosition={side === Side.RIGHT ? 0.5 : undefined}
                maxPosition={side === Side.LEFT ? 0.5 : undefined}
            />
            {side === Side.RIGHT && (
                <div
                    ref={setContentRef}
                    className={cn(hidden && "hidden")}
                >
                    {children}
                </div>
            )}
        </>
    );
}
