import { createContext, type ReactNode, useState } from "react";
import { createStore, type StoreApi } from "zustand";

export enum ToastType {
    UNKNOWN,
    INFO,
    SUCCESS,
    WARNING,
    ERROR,
}

export enum ToastPosition {
    TOP,
}

export interface Toast {
    id: PropertyKey;
    /**
     * duration of the toast, in milliseconds
     */
    duration: number;
    type: ToastType;
    pos: ToastPosition;
    render: () => ReactNode;
}

export interface IToastStore {
    pushToast(toast: Toast): void;
    genId(): PropertyKey;
    _toasts: Toast[];
}

function genId(): number {
    let id: number;

    while (!(id = Math.random()))
        ;

    return id;
}

function createToastStore(): StoreApi<IToastStore> {
    return createStore<IToastStore>((set) => ({
        _toasts: [],
        genId,
        pushToast(toast) {
            set((state) => ({
                _toasts: [...state._toasts, toast],
            }));
        },
    }));
}

export function useNewToastStore(): StoreApi<IToastStore> {
    const [store] = useState(() => createToastStore());

    return store;
}

export const ToastContext = createContext<StoreApi<IToastStore> | null>(null);

ToastContext.displayName = "ToastContext";
