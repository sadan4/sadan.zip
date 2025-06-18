import { type Modal, SYM_INTERNAL_KEY } from "..";

import _ from "lodash";
import { create } from "zustand";

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
            modals: _.initial(state.modals),
        }));
    },
    popAllModals() {
        set(() => ({
            modals: [],
        }));
    },
    popModalByKey(key: string) {
        set((state) => {
            const idx = _.findLastIndex(state.modals, { key });

            if (idx === -1)
                return {};

            return {
                modals: state.modals.toSpliced(idx, 1),
            };
        });
    },
    _popModalByInternalKey(key: symbol) {
        set((state) => {
            const idx = _.findLastIndex(state.modals, { [SYM_INTERNAL_KEY]: key });

            if (idx === -1)
                return {};

            return {
                modals: state.modals.toSpliced(idx, 1),
            };
        });
    },
}));
