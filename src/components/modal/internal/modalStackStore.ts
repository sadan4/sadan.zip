import { type Modal } from "..";

import { create } from "zustand";

export const SYM_INTERNAL_KEY = Symbol.for("modal.internal.key");

export interface ModalStackStore {
    readonly modals: readonly Readonly<Modal>[];
    pushModal(modal: Modal): void;
    popModal(): void;
    popAllModals(): void;
    popModalByKey(key: string): void;
    _popModalByInternalKey(key: symbol): void;
}

export const useModalStackStore = create<ModalStackStore>((set) => ({
    modals: [],
    pushModal(modal: Modal) {
        set((state) => ({
            modals: [...state.modals, modal],
        }));
    },
    popModal() {
        set((state) => ({
            modals: state.modals.slice(1),
        }));
    },
    popAllModals() {
        set(() => ({
            modals: [],
        }));
    },
    popModalByKey(key: string) {
        set((state) => {
            const idx = state.modals.findLastIndex((modal) => modal.key === key);

            if (idx === -1)
                return {};

            return {
                modals: state.modals.toSpliced(idx, 1),
            };
        });
    },
    _popModalByInternalKey(key: symbol) {
        set((state) => {
            const idx = state.modals.findLastIndex((x) => x[SYM_INTERNAL_KEY] === key);

            if (idx === -1)
                return {};

            return {
                modals: state.modals.toSpliced(idx, 1),
            };
        });
    },
}));
