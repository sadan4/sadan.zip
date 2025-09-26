import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { type ResizeHandleAPI, VerticalResizeHandle } from "@/components/layout/ResizeHandle";
import cn from "@/utils/cn";
import { namedContext } from "@/utils/devtools";
import error from "@/utils/error";

import { createRef, type PropsWithChildren, type RefObject, useContext, useEffect, useRef, useState } from "react";
import { createStore, type ExtractState, type StoreApi, useStore } from "zustand";

interface SidebarStateStore {
    hidden: boolean;
    /**
     * Get the current width of the sidebar.
     *
     * the value is in the range 0-1, representing the percentage of the sidebar's width relative to its container.
     */
    width: number;
    /**
     * Set the current width of the sidebar.
     *
     * the value is in the range 0-1, representing the percentage of the sidebar's width relative to its container.
     */
    setWidth(p: number): void;
    show(): void;
    hide(): void;
    sidebarApi: RefObject<ResizeHandleAPI | null>;
    ref: HTMLDivElement | null;
    setRef(ref: HTMLDivElement | null): void;
    setSidebarApi(ref?: RefObject<ResizeHandleAPI | null>): void;
}

const HIDE_THRESHOLD = 2;
const DEFAULT_WIDTH = 13;

function createSidebarStateStore(hideThreshold = HIDE_THRESHOLD) {
    return createStore<SidebarStateStore>((set, get) => ({
        hidden: false,
        width: DEFAULT_WIDTH,
        ref: null,
        sidebarApi: createRef(),
        setWidth(width) {
            const { ref } = get();

            if (ref) {
                ref.style.width = `${width}%`;
            }

            if (width < hideThreshold) {
                get().hide();
            } else if (width > hideThreshold) {
                set({ hidden: false });
            }

            set({ width });
        },
        hide() {
            set({ hidden: true });
        },
        show() {
            let { width, sidebarApi } = get();

            if (width <= hideThreshold) {
                get().setWidth(width = DEFAULT_WIDTH);
            }
            sidebarApi.current?.setCurrentPos(width);
            set({
                hidden: false,
            });
        },
        setRef(ref) {
            const { hidden, width } = get();

            if (ref) {
                if (hidden) {
                    ref.style.width = "0";
                } else {
                    ref.style.width = `${width}%`;
                }
            }
            set({ ref });
        },
        setSidebarApi(sidebarApi = createRef()) {
            set({ sidebarApi });
        },
    }));
}

const SidebarStateStoreContext = namedContext<StoreApi<SidebarStateStore> | null>(null, "SidebarStateContext");

interface SidebarStateStoreProviderProps extends PropsWithChildren {
    store?: StoreApi<SidebarStateStore>;
}

function SidebarStateStoreProvider({ children, store }: SidebarStateStoreProviderProps) {
    const storeRef = useRef<StoreApi<SidebarStateStore>>(store);

    storeRef.current ??= createSidebarStateStore();

    return (
        <SidebarStateStoreContext value={storeRef.current}>
            {children}
        </SidebarStateStoreContext>
    );
}

function useSidebarStateStore<R>(selector: (state: ExtractState<StoreApi<SidebarStateStore>>) => R): R {
    const store = useContext(SidebarStateStoreContext);

    if (!store) {
        throw new Error("useSidebarStateStore must be used within a SidebarStateStoreProvider");
    }

    return useStore(store, selector);
}

const leftSidebarStateStore = createSidebarStateStore();
const rightSidebarStateStore = createSidebarStateStore();

const enum Side {
    LEFT,
    RIGHT,
}

interface SidebarProps extends PropsWithChildren {
    side: Side;
    boundingElement: RefObject<HTMLDivElement | null>;
    defaultSize?: number;
}

function defaultInitialSize(side: Side) {
    switch (side) {
        case Side.LEFT:
            return DEFAULT_WIDTH / 100;
        case Side.RIGHT:
            return 1 - (DEFAULT_WIDTH / 100);
        default:
            error("unhandled case");
    }
}

function Sidebar({
    side,
    defaultSize = defaultInitialSize(side),
    children,
    boundingElement,
}: SidebarProps) {
    const store = useContext(SidebarStateStoreContext)!;
    const [contentRef, setContentRef] = useState<HTMLDivElement | null>(null);
    const sidebarApiRef = useRef<ResizeHandleAPI | null>(null);
    const hidden = useSidebarStateStore(({ hidden }) => hidden);
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
                    setShouldDispatch(!hidden);
                }}
                className={cn(hidden && "pointer-events-none opacity-0", shouldDispatch || "pointer-events-none")}
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
                        <Sidebar
                            boundingElement={sidebarBoundingRef}
                            side={Side.LEFT}
                        >
                            left sidebar
                        </Sidebar>
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
                        <Sidebar
                            boundingElement={sidebarBoundingRef}
                            side={Side.RIGHT}
                        >
                            right sidebar
                        </Sidebar>
                    </SidebarStateStoreProvider>
                </div>
            </div>
        </>
    );
}
