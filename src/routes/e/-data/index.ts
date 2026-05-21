import { makeLazy } from "@/utils/lazy";
import { type Monaco, monaco } from "@/utils/monaco";
import { TextmateTheme } from "@/utils/textmate/theme";
import type { Fields, TBundleHash, Thenable, TModuleId } from "@/utils/types";

import { getBuildService, type RemoteBuildService } from "./worker/api";

import "core-js/proposals/array-buffer-base64";
import z from "zod";
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface ModuleViewerStore {
    readonly buildHash: TBundleHash;
    readonly _buildService: RemoteBuildService;
    readonly _moduleModelMap: Map<TModuleId, Monaco.editor.ITextModel>;
    readonly _pendingModuleModelMap: Map<TModuleId, Promise<Monaco.editor.ITextModel>>;
    // readonly _parserMap: Map<TModuleId, WebpackAstParser>;
    // readonly _idList: Uint32Array | null;
    readonly selectedModule: number | null;
    readonly activePanel: ViewMode;
    readonly moduleSidebarOpen: boolean;
    init(newBuildHash: TBundleHash): Promise<void>;
    reset(): void;
    updateActivePanel(panel: ViewMode): void;
    updateModuleSidebarOpen(open: boolean): void;
    getModuleModel(moduleId: TModuleId): Thenable<Monaco.editor.ITextModel>;
    // getModuleParser(moduleId: TModuleId): WebpackAstParser;
    // getDepsForModule(moduleId: TModuleId): ModuleDep;
    // getAllModuleIds(): Uint32Array;
    hasId(moduleId: number): Promise<boolean>;
}

export function getModuleURI(buildHash: TBundleHash, moduleId: TModuleId) {
    return monaco.Uri.parse(`file:///bundle/${buildHash}/${moduleId}.js`);
}

export const enum ViewMode {
    CODE,
    MODULE_GRAPH,
}


function getValueDefaults(): Fields<ModuleViewerStore> {
    return {
        buildHash: "" as TBundleHash,
        _buildService: null!,
        _moduleModelMap: new Map(),
        _pendingModuleModelMap: new Map(),
        // _bundle: null,
        // _parserMap: new Map(),
        // _idList: null,
        selectedModule: null,
        activePanel: ViewMode.CODE,
        moduleSidebarOpen: true,
    };
}

export const useModuleViewerStore = create<ModuleViewerStore>((set, get) => ({
    ...getValueDefaults(),
    async init(newBuildHash) {
        const { buildHash, reset } = get();

        if (newBuildHash !== buildHash) {
            reset();
            set({
                buildHash: newBuildHash,
            });

            const _buildService = await getBuildService(newBuildHash);

            set({
                _buildService,
            });
        }
    },
    reset() {
        const { _moduleModelMap } = get();

        for (const [, model] of _moduleModelMap) {
            model.dispose();
        }

        set(getValueDefaults());
    },
    updateActivePanel(activePanel: ViewMode) {
        set({ activePanel });
    },
    updateModuleSidebarOpen(moduleSidebarOpen: boolean) {
        set({ moduleSidebarOpen });
    },
    getModuleModel(moduleId) {
        const { buildHash, _moduleModelMap, _pendingModuleModelMap, _buildService } = get();

        if (_moduleModelMap.has(moduleId)) {
            return _moduleModelMap.get(moduleId)!;
        }

        if (_pendingModuleModelMap.has(moduleId)) {
            return _pendingModuleModelMap.get(moduleId)!;
        }

        const modelPromise = async function (): Promise<Monaco.editor.ITextModel> {
            const text = await _buildService.getFormattedSource(moduleId);
            const uri = getModuleURI(buildHash, moduleId);
            const model = monaco.editor.createModel(text, "javascript", uri);

            _moduleModelMap.set(moduleId, model);
            _pendingModuleModelMap.delete(moduleId);

            return model;
        }();

        _pendingModuleModelMap.set(moduleId, modelPromise);

        return modelPromise;
    },
    // getDepsForModule(moduleId) {
    //     const { _bundle } = get();

    //     const guh = _bundle!.get_module_deps(+moduleId) ?? {
    //         syncUses: [],
    //         lazyUses: [],
    //     };

    //     return {
    //         syncUses: guh.syncUses.map(String),
    //         lazyUses: guh.lazyUses.map(String),
    //     };
    // },
    // getAllModuleIds() {
    //     const { _idList, _bundle } = get();

    //     if (_idList == null) {
    //         const idList = _bundle!.get_id_list();

    //         set({
    //             _idList: idList,
    //         });
    //         return idList;
    //     }

    //     return _idList;
    // },
    hasId(moduleId) {
        const { _buildService } = get();

        return _buildService.hasId(moduleId);
    },
}));

// make react compiler happy
export const ModuleViewerStore = useModuleViewerStore;

export const placeholderURI = makeLazy(() => monaco.Uri.parse("file:///placeholder.js"));
export const placeholderModel = makeLazy(() => {
    const uri = placeholderURI();

    // during HMR, we can't create the same model twice, so we have to check if it's already created
    if (import.meta.env.DEV) {
        const existingModel = monaco.editor.getModel(uri);

        if (existingModel) {
            return existingModel;
        }
    }

    return monaco.editor.createModel("", "javascript", uri);
});

const MODULE_URI_REGEX = /^file:\/\/\/bundle\/([^/]+?)\/([^/.]+?)\.js$/;

export interface ParsedModuleURI {
    buildHash: TBundleHash;
    moduleId: TModuleId;
}

export function parseModuleURI(uri: Monaco.Uri): ParsedModuleURI | undefined {
    const match = MODULE_URI_REGEX.exec(uri.toString());

    if (!match) {
        return undefined;
    }

    const [, buildHash, moduleId] = match;

    return {
        buildHash: buildHash as TBundleHash,
        moduleId: +moduleId as TModuleId,
    };
}

const IModuleViewerSettings = z.object({
    openModulesInNewTab: z.boolean().catch(false),
    editorTheme: z.enum(TextmateTheme).catch(TextmateTheme.TOKYO_NIGHT),
});

export type IModuleViewerSettings = z.infer<typeof IModuleViewerSettings>;

export const useModuleViewerSettingsStore = create<IModuleViewerSettings>()(persist(() => ({
    openModulesInNewTab: false,
    editorTheme: TextmateTheme.TOKYO_NIGHT,
} satisfies IModuleViewerSettings as IModuleViewerSettings), {
    name: "module-viewer-settings",
    version: 1,
    onRehydrateStorage() {
        return (_state, error) => {
            if (error || !_state) {
                return;
            }

            const state = IModuleViewerSettings.parse(_state);

            Object.assign(_state, state);
        };
    },
    skipHydration: import.meta.env.SSR,
}));

// make react compiler happy
export const ModuleViewerSettingsStore = useModuleViewerSettingsStore;
