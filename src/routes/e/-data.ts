import { sendMessage } from "@/utils/e/socket";
import { assert } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
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
    init(newBuildHash: string): void;
    reset(): void;
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

const getValueDefaults = () => ({
    buildHash: "",
    moduleCodeMap: new Map(),
    moduleModelMap: new Map(),
    _pendingModels: new Map<string, Promise<Monaco.editor.ITextModel>>(),
    parserMap: new Map<string, WebpackAstParser>(),
    _pendingParsers: new Map<string, Promise<WebpackAstParser>>(),
    selectedModule: null,
    allModuleIds: [],
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
        const { moduleModelMap } = get();

        for (const [, model] of moduleModelMap) {
            model.dispose();
        }

        set(getValueDefaults());
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
        const { getModuleParser, moduleModelMap, buildHash, _pendingModels: pendingModels } = get();

        if (moduleModelMap.has(moduleId)) {
            return moduleModelMap.get(moduleId)!;
        }

        if (pendingModels.has(moduleId)) {
            return pendingModels.get(moduleId)!;
        }

        const promise = (async () => {
            const code = (await getModuleParser(moduleId)).text;
            const uri = getModuleURI(buildHash, moduleId);
            const model = monaco.editor.createModel(code, "javascript", uri);

            moduleModelMap.set(moduleId, model);
            pendingModels.delete(moduleId);

            return model;
        })();

        pendingModels.set(moduleId, promise);

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
        const { parserMap, getModuleCode, _pendingParsers } = get();

        if (parserMap.has(moduleId)) {
            return parserMap.get(moduleId)!;
        }

        if (_pendingParsers.has(moduleId)) {
            return _pendingParsers.get(moduleId)!;
        }

        const promise = (async () => {
            const code = await getModuleCode(moduleId);
            const parser = WebpackAstParser.withFormattedModule(code, moduleId);

            parserMap.set(moduleId, parser);
            _pendingParsers.delete(moduleId);

            return parser;
        })();

        _pendingParsers.set(moduleId, promise);

        return promise;
    },
}));

// make react compiler happy
export const ModuleViewerStore = useModuleViewerStore;
