import { create } from "zustand";

interface CursorOpacityStore {
    currentOpacity: number;
}

export const useCursorOpacityStore = create<CursorOpacityStore>((set) => ({
    currentOpacity: 1,
    fadeIn() {
        set((state) => ({
            ...state,
            currentOpacity: 1,
        }));
    },
    fadeOut() {
        set((state) => ({
            ...state,
            currentOpacity: 0,
        }));
    },
}));
