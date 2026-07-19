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
    BOTTOM,
}

export interface Toast {
    id: PropertyKey;
    /**
     * duration of the toast, in milliseconds
     */
    duration: number;
    type: ToastType;
    pos: ToastPosition;
    render(): ReactNode;
}

export interface IToastStore {
    pushToast(toast: Toast): void;
    popToast(id?: PropertyKey): void;
    genId(): PropertyKey;
    _toasts: Toast[];
}

let id = 0;

function genId(): number {
    return ++id;
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
        popToast(id?: PropertyKey) {
            if (id === undefined) {
                set(({ _toasts: [, ..._toasts] }) => ({ _toasts }));
            } else {
                set(({ _toasts }) => {
                    const idx = _toasts.findIndex((t) => t.id === id);

                    if (idx !== -1) {
                        return {
                            _toasts: _toasts.toSpliced(idx, 1),
                        };
                    }

                    return {};
                });
            }
        },
    }));
}

export function useNewToastStore(): StoreApi<IToastStore> {
    const [store] = useState(() => createToastStore());

    return store;
}

export const ToastContext = createContext<StoreApi<IToastStore> | null>(null);

ToastContext.displayName = "ToastContext";
