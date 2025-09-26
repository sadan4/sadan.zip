import { createSidebarStateStore, type SidebarStateStore, SidebarStateStoreContext, type SidebarStateStoreProviderProps } from "./store";

import { useRef } from "react";
import type { StoreApi } from "zustand";

export function SidebarStateStoreProvider({ children, store }: SidebarStateStoreProviderProps) {
    const storeRef = useRef<StoreApi<SidebarStateStore>>(store);

    storeRef.current ??= createSidebarStateStore();

    return (
        <SidebarStateStoreContext value={storeRef.current}>
            {children}
        </SidebarStateStoreContext>
    );
}
