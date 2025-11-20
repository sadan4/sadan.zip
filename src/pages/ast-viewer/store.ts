import { createSidebarStateStore } from "@/components/layout/ResizableSidebar";
import { debounce } from "@/utils/functional";
import { pick } from "@/utils/obj";
import { Language } from "@/utils/textmate";
import { TextmateTheme } from "@/utils/textmate/theme";
import { TreeMode } from "@/utils/typescript";
import { createSelectors } from "@/utils/zustand";

import { create } from "zustand";
import { persist } from "zustand/middleware";

export const leftAstSidebarStateStore = createSidebarStateStore();
export const rightAstSidebarStateStore = createSidebarStateStore();

export interface ASTViewerStore {
    code: string;
    language: Language;
    theme: TextmateTheme;
    treeMode: TreeMode;
    /**
     * Not debounced
     *
     * for the debounced version, see {@link updateASTViewerCode}
     */
    updateCode(newCode: string): void;
}

// TODO: inline and replace with store.getInitialState() ?
const DEFAULT_AST_VIEWER_STATE: Pick<ASTViewerStore, "code" | "language" | "theme" | "treeMode"> = {
    code: "",
    language: Language.TYPESCRIPT_REACT,
    theme: TextmateTheme.TOKYO_NIGHT,
    treeMode: TreeMode.GET_CHILDREN,
};

export const useASTViewerStore = createSelectors(create<ASTViewerStore>()(persist(
    (set, _get) => ({
        ...DEFAULT_AST_VIEWER_STATE,
        updateCode(code: string) {
            set({ code });
        },
    }),
    {
        name: "ast-viewer-store",
        version: 1,
        partialize(state) {
            return pick(state, ["code", "language", "theme"]);
        },
        onRehydrateStorage(_state) {
            return (state, error) => {
                if (error) {
                    console.error("failed to rehydrate ast viewer store:", error);
                    return;
                }
                if (!state) {
                    return;
                }
                if (typeof state.code !== "string") {
                    console.warn("invalid value for state.code, defaulting");
                    state.updateCode(DEFAULT_AST_VIEWER_STATE.code);
                }
                if (!Object.values(Language).includes(state.language)) {
                    console.warn("invalid value for state.language, defaulting");
                    state.language = DEFAULT_AST_VIEWER_STATE.language;
                }
                if (!Object.values(TreeMode).includes(state.treeMode)) {
                    console.warn("invalid value for state.treeMode, defaulting");
                    state.treeMode = DEFAULT_AST_VIEWER_STATE.treeMode;
                }
                if (typeof state.theme !== "number" || !Object.hasOwn(TextmateTheme, state.theme)) {
                    console.warn("invalid value for state.theme, defaulting");
                    state.theme = DEFAULT_AST_VIEWER_STATE.theme;
                }
            };
        },
    },
)));

export const updateASTViewerCode = debounce((newCode: string) => {
    useASTViewerStore.getState().updateCode(newCode);
}, 750);
