import { create } from "zustand";

export interface BundleDownloadProgress {
    stage: "downloading";
    /**
     * bytes received so far
     */
    loaded: number;
    /**
     * total bytes, or null if unknown
     */
    total: number | null;
}

export interface BundleProcessingProgress {
    stage: "processing";
}

export type BundleLoadProgress = BundleDownloadProgress | BundleProcessingProgress;

interface BundleLoadStore {
    readonly progress: BundleLoadProgress | null;
    setProgress(progress: BundleLoadProgress | null): void;
}

/**
 * Progress of the bundle currently being loaded by `ModuleViewerStore.init`.
 *
 * Kept separate from `-data/index.ts` so the route's loading screen can use it without pulling in monaco.
 */
export const useBundleLoadStore = create<BundleLoadStore>((set) => ({
    progress: null,
    setProgress(progress) {
        set({ progress });
    },
}));

export type DiffSide = "a" | "b";

interface DiffLoadStore {
    readonly progress: Readonly<Record<DiffSide, BundleLoadProgress | null>>;
    setProgress(side: DiffSide, progress: BundleLoadProgress | null): void;
}

/**
 * Progress of the two bundles being loaded by the `/e/diff` route.
 */
export const useDiffLoadStore = create<DiffLoadStore>((set) => ({
    progress: {
        a: null,
        b: null,
    },
    setProgress(side, progress) {
        set((state) => ({
            progress: {
                ...state.progress,
                [side]: progress,
            },
        }));
    },
}));
