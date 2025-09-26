import { namedContext } from "@/utils/devtools";
import error from "@/utils/error";

import type { ResizeHandleAPI } from "../ResizeHandle";

import { createRef, type PropsWithChildren, type RefObject, useContext } from "react";
import { createStore, type ExtractState, type StoreApi, useStore } from "zustand";

export interface SidebarStateStore {
    hidden: boolean;
    handleHidden: boolean;
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

export const HIDE_THRESHOLD = 6;
export const DEFAULT_WIDTH = 13;

export function createSidebarStateStore(hideThreshold = HIDE_THRESHOLD) {
    return createStore<SidebarStateStore>((set, get) => ({
        hidden: false,
        handleHidden: false,
        width: DEFAULT_WIDTH,
        ref: null,
        sidebarApi: createRef(),
        setWidth(width) {
            const { ref } = get();

            if (ref) {
                ref.style.width = `${width}%`;
            }

            let handleHidden = false;

            if (width < hideThreshold) {
                if (width > (hideThreshold / 2)) {
                    width = hideThreshold;
                    if (ref) {
                        ref.style.width = `${width}%`;
                    }
                    handleHidden = true;
                    set({
                        hidden: false,
                    });
                } else {
                    get().hide();
                }
            } else if (width > hideThreshold) {
                set({ hidden: false });
            }

            set({
                width,
                handleHidden,
            });
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

export const SidebarStateStoreContext = namedContext<StoreApi<SidebarStateStore> | null>(null, "SidebarStateContext");

export interface SidebarStateStoreProviderProps extends PropsWithChildren {
    store?: StoreApi<SidebarStateStore>;
}

export function useSidebarStateStore<R>(selector: (state: ExtractState<StoreApi<SidebarStateStore>>) => R): R {
    const store = useContext(SidebarStateStoreContext);

    if (!store) {
        throw new Error("useSidebarStateStore must be used within a SidebarStateStoreProvider");
    }

    return useStore(store, selector);
}

export const enum Side {
    LEFT,
    RIGHT,
}

export function defaultInitialSize(side: Side) {
    switch (side) {
        case Side.LEFT:
            return DEFAULT_WIDTH / 100;
        case Side.RIGHT:
            return 1 - (DEFAULT_WIDTH / 100);
        default:
            error("unhandled case");
    }
}
