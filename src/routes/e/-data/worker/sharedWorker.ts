/// <reference lib="webworker" />

import type { GeneratedGraph } from "@/hooks/moduleGraph2";
import { assert } from "@/utils/error";
import type { Monaco } from "@/utils/monaco";
import type { TBundleHash, TModuleId } from "@/utils/types";
import {
    Bundle,
    type BundleSearchLocation,
    type BundleSearchResultInfo,
    default as initWasm,
    type ExportTreeNode,
    get_bundle,
    HoverInfo as RawHoverInfo,
    LaidOutGraph,
    ModuleLocation as RawModuleLocation,
    MonacoPosition,
    MonacoRange,
} from "@sadan4/libsadancore";
import type { Edge, Node } from "@xyflow/react";

import * as comlink from "comlink";

export interface ModuleLocation {
    range: Monaco.IRange;
    id: TModuleId;
}

export interface HoverInfo {
    content: string;
    range: Monaco.IRange;
    i18nKey?: string;
}

export interface BundleSearchResults {
    moduleIds: Uint32Array;
    rawIndices: Uint32Array;
}

export interface ModuleDeps {
    syncUses: TModuleId[];
    lazyUses: TModuleId[];
}

export interface IBuildService {
    hasId(moduleId: number): moduleId is TModuleId;
    getFormattedSource(moduleId: TModuleId): string;
    generateDefinitions(moduleId: TModuleId, position: Monaco.IPosition): ModuleLocation[];
    generateReferences(moduleId: TModuleId, position: Monaco.IPosition): ModuleLocation[];
    generateHover(moduleId: TModuleId, position: Monaco.IPosition): HoverInfo | undefined;
    getAllModuleIds(): Uint32Array;
    searchModules(query: string, regex: boolean): BundleSearchResults;
    getSearchResultInfo(moduleId: TModuleId, rawIndex: number, longPreview: boolean): BundleSearchResultInfo;
    getSearchLocation(moduleId: TModuleId, rawIndex: number): BundleSearchLocation;
    generateModuleGraph(moduleId: TModuleId, depth: number): GeneratedGraph;
    getModuleExportMap(moduleId: TModuleId): ExportTreeNode[];
    getModuleDependencies(moduleId: TModuleId): ModuleDeps;
    getModuleDependents(moduleId: TModuleId): ModuleDeps | undefined;
}

const self = globalThis as any as SharedWorkerGlobalScope;

function convertRange({
    start: {
        line: startLineNumber,
        column: startColumn,
    },
    end: {
        line: endLineNumber,
        column: endColumn,
    },
}: MonacoRange): Monaco.IRange {
    return {
        startLineNumber,
        startColumn,
        endLineNumber,
        endColumn,
    };
}

function convertModuleLocation({ id, range }: RawModuleLocation): ModuleLocation {
    return {
        id: id as TModuleId,
        range: convertRange(range),
    };
}

function convertHoverInfo({ content, range, i18n_key: i18nKey }: RawHoverInfo): HoverInfo {
    return {
        content,
        range: convertRange(range),
        i18nKey,
    };
}

function convertGraph(raw: LaidOutGraph): GeneratedGraph {
    const nodes: Node[] = raw.nodes.map(({ id, width, height, x, y }) => ({
        id: `${id}`,
        data: {
            label: `${id}`,
        },
        position: {
            x,
            y,
        },
        width,
        height,
    }));

    const edges: Edge[] = raw.edges.map(({ from, to }) => ({
        id: `${from}->${to}`,
        source: `${from}`,
        target: `${to}`,
    }));

    return {
        nodes,
        edges,
    };
}

class BuildService implements IBuildService {
    #bundleHash!: TBundleHash;
    #bundle!: Bundle;

    public async init(hash: TBundleHash) {
        if (this.#bundleHash) {
            assert(this.#bundleHash === hash, "Worker already initialized with a different bundle hash");
        }
        this.#bundleHash = hash;
        if (this.#bundle == null) {
            await this.#downloadBundle();
        }
    }

    async #downloadBundle() {
        await initWasm();
        this.#bundle = await get_bundle(this.#bundleHash, true);
    }

    public hasId(moduleId: number): moduleId is TModuleId {
        return this.#bundle.has_id(moduleId);
    }

    public getFormattedSource(moduleId: TModuleId): string {
        return this.#bundle.get_module_text(moduleId);
    }

    generateDefinitions(moduleId: TModuleId, position: Monaco.IPosition) {
        const mPos = new MonacoPosition(position.lineNumber, position.column);
        const rawDefs = this.#bundle.provide_definition(moduleId, mPos);

        return rawDefs.map(convertModuleLocation);
    }

    generateReferences(moduleId: TModuleId, position: Monaco.IPosition) {
        const mPos = new MonacoPosition(position.lineNumber, position.column);
        const rawRefs = this.#bundle.provide_references(moduleId, mPos);

        return rawRefs.map(convertModuleLocation);
    }

    generateHover(moduleId: TModuleId, position: Monaco.IPosition) {
        const mPos = new MonacoPosition(position.lineNumber, position.column);
        const hoverInfo = this.#bundle.provide_hover(moduleId, mPos);

        return hoverInfo && convertHoverInfo(hoverInfo);
    }

    getAllModuleIds(): Uint32Array {
        const ids = this.#bundle.get_id_list();

        return comlink.transfer(ids, [ids.buffer]);
    }

    public searchModules(query: string, regex: boolean): BundleSearchResults {
        const results = this.#bundle.search_modules(query, regex) as BundleSearchResults;
        const { moduleIds } = results;
        const { rawIndices } = results;

        return comlink.transfer({
            moduleIds,
            rawIndices,
        }, [moduleIds.buffer, rawIndices.buffer]);
    }

    public getSearchResultInfo(moduleId: TModuleId, rawIndex: number, longPreview: boolean): BundleSearchResultInfo {
        return this.#bundle.get_search_result_info(moduleId, rawIndex, longPreview);
    }

    public getSearchLocation(moduleId: TModuleId, rawIndex: number): BundleSearchLocation {
        return this.#bundle.get_search_location(moduleId, rawIndex);
    }

    public generateModuleGraph(moduleId: TModuleId, depth: number): GeneratedGraph {
        const graph = this.#bundle.gen_graph(moduleId, depth);

        return convertGraph(graph);
    }

    public getModuleExportMap(moduleId: TModuleId): ExportTreeNode[] {
        return this.#bundle.get_module_export_map(moduleId);
    }

    public getModuleDependencies(moduleId: TModuleId): ModuleDeps {
        return this.#bundle.get_module_dependencies(moduleId) as ModuleDeps;
    }

    public getModuleDependents(moduleId: TModuleId): ModuleDeps | undefined {
        return this.#bundle.get_module_deps(moduleId) as ModuleDeps | undefined;
    }
}

// use a type alias to avoid including BuildService in auto imports
export type RawBuildService = BuildService;
{
    const service = new BuildService();

    self.onconnect = ({ ports: [port] }) => {
        comlink.expose(service, port);
    };
}
