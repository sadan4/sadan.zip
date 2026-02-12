import { sendMessage } from "@/utils/e/socket";
import { assert } from "@/utils/error";
import { makeLazy } from "@/utils/lazy";
import { type Monaco, monaco } from "@/utils/monaco";
import { defer } from "@/utils/scope";
import { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";

import { create } from "zustand";

interface ModuleViewerStore {
    readonly buildHash: string;
    /**
     * moduleId -> code
     * 
     * the code is unformatted
     */
    readonly moduleCodeMap: Map<string, string>;
    readonly moduleModelMap: Map<string, Monaco.editor.ITextModel>;
    readonly _pendingModels: Map<string, Promise<Monaco.editor.ITextModel>>;
    readonly parserMap: Map<string, WebpackAstParser>;
    readonly _pendingParsers: Map<string, Promise<WebpackAstParser>>;
    readonly selectedModule: string | null;
    readonly allModuleIds: string[];
    readonly activePanel: ViewMode;
    readonly moduleSidebarOpen: boolean;
    readonly _abort: AbortController;
    init(newBuildHash: string): void;
    reset(): void;
    updateActivePanel(panel: ViewMode): void;
    updateModuleSidebarOpen(open: boolean): void;
    /**
     * the code is unformatted
     */
    getModuleCode(moduleId: string): Promise<string>;
    getModuleModelSync(moduleId: string): Monaco.editor.ITextModel;
    getModuleModel(moduleId: string): Promise<Monaco.editor.ITextModel>;
    getModuleParserSync(moduleId: string): WebpackAstParser;
    getModuleParser(moduleId: string): Promise<WebpackAstParser>;
}

export function getModuleURI(buildHash: string, moduleId: string) {
    return monaco.Uri.parse(`file:///bundle/${buildHash}/${moduleId}.js`);
}

export const enum ViewMode {
    CODE,
    MODULE_GRAPH,
}


const getValueDefaults = () => ({
    buildHash: "",
    moduleCodeMap: new Map(),
    moduleModelMap: new Map(),
    _pendingModels: new Map<string, Promise<Monaco.editor.ITextModel>>(),
    parserMap: new Map<string, WebpackAstParser>(),
    _pendingParsers: new Map<string, Promise<WebpackAstParser>>(),
    selectedModule: null,
    allModuleIds: [],
    activePanel: ViewMode.CODE,
    moduleSidebarOpen: true,
    _abort: new AbortController(),
});

export const useModuleViewerStore = create<ModuleViewerStore>((set, get) => ({
    ...getValueDefaults(),
    init(newBuildHash: string) {
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
    async getModuleCode(moduleId: string) {
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
    async getModuleParser(moduleId: string) {
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
}));

// make react compiler happy
export const ModuleViewerStore = useModuleViewerStore;

export const placeholderURI = makeLazy(() => monaco.Uri.parse("file:///placeholder.js"));
export const placeholderModel = makeLazy(() => monaco.editor.createModel("", "javascript", placeholderURI()));
