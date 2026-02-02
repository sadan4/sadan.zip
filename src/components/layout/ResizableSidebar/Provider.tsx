import { useChange } from "@/hooks/change";

import { createSidebarStateStore, type SidebarStateStore, SidebarStateStoreContext, type SidebarStateStoreProviderProps } from "./store";

import { useState } from "react";
import type { StoreApi } from "zustand";

export function SidebarStateStoreProvider({ children, store }: SidebarStateStoreProviderProps) {
    const [state, setState] = useState<StoreApi<SidebarStateStore>>(() => store ?? createSidebarStateStore());

    useChange((_, cur) => {
        if (cur) {
            setState(cur);
        }
    }, store);

    return (
        <SidebarStateStoreContext value={state}>
            {children}
        </SidebarStateStoreContext>
    );
}
