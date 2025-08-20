// for type extension
import type { } from "@redux-devtools/extension";

import { type Modal } from "..";

import { create } from "zustand";
import { devtools } from "zustand/middleware";

export const SYM_INTERNAL_KEY = Symbol.for("modal.internal.key");

export interface ModalStackStore {
    readonly modals: readonly Readonly<Modal>[];
    pushModal(modal: Modal): void;
    popModal(): void;
    popAllModals(): void;
    popModalByKey(key: string): void;
    _popModalByInternalKey(key: symbol): void;
}

export const useModalStackStore = create<ModalStackStore>()(devtools((set) => ({
    modals: [],
    pushModal(modal: Modal) {
        set((state) => ({
            modals: [...state.modals, modal],
        }), undefined, "modalStack/pushModal");
    },
    popModal() {
        set((state) => ({
            modals: state.modals.slice(1),
        }), undefined, "modalStack/popModal");
    },
    popAllModals() {
        set(() => ({
            modals: [],
        }), undefined, "modalStack/popAllModals");
    },
    popModalByKey(key: string) {
        set((state) => {
            const idx = state.modals.findLastIndex((modal) => modal.key === key);

            if (idx === -1)
                return {};

            return {
                modals: state.modals.toSpliced(idx, 1),
            };
        }, undefined, "modalStack/popModalByKey");
    },
    _popModalByInternalKey(key: symbol) {
        set((state) => {
            const idx = state.modals.findLastIndex((x) => x[SYM_INTERNAL_KEY] === key);

            if (idx === -1)
                return {};

            return {
                modals: state.modals.toSpliced(idx, 1),
            };
        }, undefined, "modalStack/_popModalByInternalKey");
    },
}), {
    name: "ModalStackStore",
    store: "ModalStackStore",
    enabled: import.meta.env.DEV,
    trace: true,
}));
