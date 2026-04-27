import { type IToastStore, ToastContext } from "@/stores/ToastStore";
import { error } from "@/utils/error";

import { use } from "react";
import type { StoreApi } from "zustand";

export function useToaster(): StoreApi<IToastStore> {
    const store = use(ToastContext);

    if (store == null) {
        error("not in a toast container");
    } else {
        return store;
    }
}
