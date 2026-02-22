import { sendMessage } from "@/utils/e/socket";
import { assert } from "@/utils/error";
import { makeLazy } from "@/utils/lazy";
import { type Monaco, monaco } from "@/utils/monaco";
import { defer } from "@/utils/scope";
import type { Fields, Thenable } from "@/utils/types";
import { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";

import type { DepsJson, TBundleHash, TModuleId } from "../../../server/types";

import z from "zod";
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface ModuleViewerStore {
    readonly buildHash: TBundleHash;
    /**
     * moduleId -> code
     * 
     * the code is unformatted
     */
    readonly moduleCodeMap: Map<TModuleId, string>;
    readonly moduleModelMap: Map<TModuleId, Monaco.editor.ITextModel>;
    readonly _pendingModels: Map<TModuleId, Promise<Monaco.editor.ITextModel>>;
    readonly parserMap: Map<TModuleId, WebpackAstParser>;
    readonly _pendingParsers: Map<TModuleId, Promise<WebpackAstParser>>;
    readonly selectedModule: TModuleId | null;
    readonly allModuleIds: TModuleId[];
    readonly activePanel: ViewMode;
    readonly moduleSidebarOpen: boolean;
    readonly _abort: AbortController;
    readonly _pendingDepsGraph: Promise<DepsJson> | null;
    readonly _depsGraph: DepsJson | null;
    init(newBuildHash: TBundleHash): void;
    reset(): void;
    updateActivePanel(panel: ViewMode): void;
    updateModuleSidebarOpen(open: boolean): void;
    /**
     * the code is unformatted
     */
    getModuleCode(moduleId: TModuleId): Promise<string>;
    getModuleModelSync(moduleId: TModuleId): Monaco.editor.ITextModel;
    getModuleModel(moduleId: TModuleId): Promise<Monaco.editor.ITextModel>;
    getModuleParserSync(moduleId: TModuleId): WebpackAstParser;
    getModuleParser(moduleId: TModuleId): Promise<WebpackAstParser>;
    getDepsGraph(): Thenable<DepsJson>;
}

export function getModuleURI(buildHash: TBundleHash, moduleId: TModuleId) {
    return monaco.Uri.parse(`file:///bundle/${buildHash}/${moduleId}.js`);
}

export const enum ViewMode {
    CODE,
    MODULE_GRAPH,
}


const getValueDefaults = (): Fields<ModuleViewerStore> => ({
    buildHash: "" as TBundleHash,
    moduleCodeMap: new Map(),
    moduleModelMap: new Map(),
    _pendingModels: new Map(),
    parserMap: new Map(),
    _pendingParsers: new Map(),
    selectedModule: null,
    allModuleIds: [],
    activePanel: ViewMode.CODE,
    moduleSidebarOpen: true,
    _abort: new AbortController(),
    _pendingDepsGraph: null,
    _depsGraph: null,
});

export const useModuleViewerStore = create<ModuleViewerStore>((set, get) => ({
    ...getValueDefaults(),
    init(newBuildHash) {
        const { buildHash, reset } = get();

        if (newBuildHash !== buildHash) {
            reset();
            set({
                buildHash: newBuildHash,
            });
        }
    },
    reset() {
        const { moduleModelMap, _abort } = get();

        _abort.signal.throwIfAborted();

        for (const [, model] of moduleModelMap) {
            model.dispose();
        }

        _abort.abort();

        set(getValueDefaults());
    },
    updateActivePanel(activePanel: ViewMode) {
        set({ activePanel });
    },
    updateModuleSidebarOpen(moduleSidebarOpen: boolean) {
        set({ moduleSidebarOpen });
    },
    async getModuleCode(moduleId) {
        const { moduleCodeMap, buildHash } = get();

        if (moduleCodeMap.has(moduleId)) {
            return moduleCodeMap.get(moduleId)!;
        }

        const { fileText } = await sendMessage<"getBundleFileResponse">({
            type: "getBundleFile",
            bundleHash: buildHash,
            moduleNumber: moduleId,
        });

        moduleCodeMap.set(moduleId, fileText);

        return fileText;
    },
    getModuleModelSync(moduleId) {
        const { getModuleParserSync, moduleModelMap, buildHash } = get();

        if (moduleModelMap.has(moduleId)) {
            return moduleModelMap.get(moduleId)!;
        }

        const { text } = getModuleParserSync(moduleId);
        const uri = getModuleURI(buildHash, moduleId);
        const model = monaco.editor.createModel(text, "javascript", uri);

        moduleModelMap.set(moduleId, model);

        return model;
    },
    async getModuleModel(moduleId) {
        const { getModuleParser, moduleModelMap, buildHash, _pendingModels, _abort } = get();

        if (moduleModelMap.has(moduleId)) {
            return moduleModelMap.get(moduleId)!;
        }

        if (_pendingModels.has(moduleId)) {
            return _pendingModels.get(moduleId)!;
        }

        const promise = (async () => {
            using _ = defer(() => {
                _pendingModels.delete(moduleId);
            });

            const code = (await getModuleParser(moduleId)).text;
            const uri = getModuleURI(buildHash, moduleId);

            _abort.signal.throwIfAborted();

            const model = monaco.editor.createModel(code, "javascript", uri);

            moduleModelMap.set(moduleId, model);

            return model;
        })();

        _pendingModels.set(moduleId, promise);

        return promise;
    },
    getModuleParserSync(moduleId) {
        const { parserMap, moduleCodeMap } = get();

        if (parserMap.has(moduleId)) {
            return parserMap.get(moduleId)!;
        }

        assert(moduleCodeMap.has(moduleId), "no code to make the parser from");

        const code = moduleCodeMap.get(moduleId)!;
        const parser = WebpackAstParser.withFormattedModule(code, moduleId);

        parserMap.set(moduleId, parser);

        return parser;
    },
    async getModuleParser(moduleId) {
        const { parserMap, getModuleCode, _pendingParsers, _abort } = get();

        if (parserMap.has(moduleId)) {
            return parserMap.get(moduleId)!;
        }

        if (_pendingParsers.has(moduleId)) {
            return _pendingParsers.get(moduleId)!;
        }

        const promise = (async () => {
            using _ = defer(() => {
                _pendingParsers.delete(moduleId);
            });

            const code = await getModuleCode(moduleId);

            _abort.signal.throwIfAborted();

            const parser = WebpackAstParser.withFormattedModule(code, moduleId);

            parserMap.set(moduleId, parser);

            return parser;
        })();

        _pendingParsers.set(moduleId, promise);

        return promise;
    },
    getDepsGraph(): Thenable<DepsJson> {
        const { _depsGraph, buildHash, _pendingDepsGraph } = get();

        if (_depsGraph) {
            return _depsGraph;
        }

        if (_pendingDepsGraph) {
            return _pendingDepsGraph;
        }

        const p = sendMessage<"getBundleDepGraphResponse">({
            type: "getBundleDepGraph",
            bundleHash: buildHash,
        }).then(({ depGraph }) => {
            set({ _depsGraph: depGraph });
            return depGraph;
        });

        set({ _pendingDepsGraph: p });

        return p;
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
        moduleId: moduleId as TModuleId,
    };
}

const IModuleViewerSettings = z.object({
    openModulesInNewTab: z.boolean().catch(false),
});

export type IModuleViewerSettings = z.infer<typeof IModuleViewerSettings>;

export const useModuleViewerSettingsStore = create<IModuleViewerSettings>()(persist(() => ({
    openModulesInNewTab: false as boolean,
}), {
    name: "module-viewer-settings",
    version: 1,
    onRehydrateStorage(_state) {
        const state = IModuleViewerSettings.parse(_state);

        Object.assign(_state, state);
    },
    skipHydration: import.meta.env.SSR,
}));

// make react compiler happy
export const ModuleViewerSettingsStore = useModuleViewerSettingsStore;
