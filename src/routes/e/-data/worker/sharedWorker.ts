/// <reference lib="webworker" />

import type { GeneratedGraph } from "@/hooks/moduleGraph2";
import { assert } from "@/utils/error";
import type { Monaco } from "@/utils/monaco";
import type { TBundleHash, TModuleId } from "@/utils/types";
import {
    type Bundle,
    type BundleSearchLocation,
    type BundleSearchResultInfo,
    default as initWasm,
    type ExperimentInfo,
    type ExportTreeNode,
    full_bundle_endpoint,
    type HoverInfo as RawHoverInfo,
    type LaidOutGraph,
    type ModuleLocation as RawModuleLocation,
    MonacoPosition,
    type MonacoRange,
    parse_bundle,
} from "@sadan4/libsadancore";
import type { Edge, Node } from "@xyflow/react";

import type { BundleLoadProgress } from "../loadProgress";

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

export interface IIcon {
    name?: string;
    svg: string;
    pos: Monaco.IRange;
    definedIn: TModuleId;
}

export interface IBuildService {
    hasId(moduleId: number): moduleId is TModuleId;
    getFormattedSource(moduleId: TModuleId): string;
    generateDefinitions(moduleId: TModuleId, position: Monaco.IPosition): Promise<ModuleLocation[]>;
    generateReferences(moduleId: TModuleId, position: Monaco.IPosition): Promise<ModuleLocation[]>;
    generateHover(moduleId: TModuleId, position: Monaco.IPosition): Promise<HoverInfo | undefined>;
    getAllModuleIds(): Uint32Array;
    searchModules(query: string, regex: boolean): BundleSearchResults;
    getSearchResultInfo(moduleId: TModuleId, rawIndex: number, longPreview: boolean): BundleSearchResultInfo;
    getSearchLocation(moduleId: TModuleId, rawIndex: number): BundleSearchLocation;
    generateModuleGraph(moduleId: TModuleId, depth: number): GeneratedGraph;
    getModuleExportMap(moduleId: TModuleId): ExportTreeNode[];
    getModuleDependencies(moduleId: TModuleId): ModuleDeps;
    getModuleDependents(moduleId: TModuleId): ModuleDeps | undefined;
    getExperiments(): ExperimentInfo[];
    getIcons(): Promise<IIcon[]>;
}

const self = globalThis as any as SharedWorkerGlobalScope;

export type BundleLoadProgressCallback = (progress: BundleLoadProgress) => void;

/**
 * minimum time between download progress reports, in ms
 */
const PROGRESS_INTERVAL = 50;

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
    /**
     * shared between all tabs connected to this worker, so the bundle is only downloaded once
     */
    #initPromise: Promise<void> | null = null;
    readonly #progressListeners = new Set<BundleLoadProgressCallback>();
    #lastProgress: BundleLoadProgress | null = null;

    public async init(hash: TBundleHash, onProgress?: BundleLoadProgressCallback) {
        if (this.#bundleHash) {
            assert(this.#bundleHash === hash, "Worker already initialized with a different bundle hash");
        }
        this.#bundleHash = hash;
        // oxlint-disable-next-line typescript/no-unnecessary-condition
        if (this.#bundle != null) {
            return;
        }
        if (onProgress) {
            this.#progressListeners.add(onProgress);
            if (this.#lastProgress) {
                onProgress(this.#lastProgress);
            }
        }
        try {
            this.#initPromise ??= this.#downloadBundle().catch((e: unknown) => {
                // allow retrying
                this.#initPromise = null;
                throw e;
            });
            await this.#initPromise;
        } finally {
            if (onProgress) {
                this.#progressListeners.delete(onProgress);
            }
        }
    }

    #reportProgress(progress: BundleLoadProgress) {
        this.#lastProgress = progress;
        for (const listener of this.#progressListeners) {
            listener(progress);
        }
    }

    async #downloadBundle() {
        await initWasm();

        const rsp = await fetch(full_bundle_endpoint(this.#bundleHash));

        if (!rsp.ok) {
            throw new Error(`Failed to fetch bundle ${this.#bundleHash}: ${rsp.status} ${rsp.statusText}`);
        }

        const data = await this.#readWithProgress(rsp);

        this.#reportProgress({ stage: "processing" });
        this.#bundle = parse_bundle(data, true);
        this.#lastProgress = null;
    }

    async #readWithProgress(rsp: Response): Promise<Uint8Array> {
        const encoding = rsp.headers.get("Content-Encoding");
        // Content-Length is wire size, but the stream is decoded
        const isEncoded = encoding != null && encoding !== "identity";
        const lengthHeader = Number(rsp.headers.get("Content-Length"));
        let total = !isEncoded && Number.isFinite(lengthHeader) && lengthHeader > 0 ? lengthHeader : null;
        let loaded = 0;

        this.#reportProgress({
            stage: "downloading",
            loaded,
            total,
        });

        if (!rsp.body) {
            return new Uint8Array(await rsp.arrayBuffer());
        }

        const chunks: Uint8Array[] = [];
        const reader = rsp.body.getReader();
        let lastReport = 0;

        for (;;) {
            const { done, value } = await reader.read();

            if (done) {
                break;
            }
            chunks.push(value);
            loaded += value.byteLength;
            // if the response is content-encoded, Content-Length is the encoded size,
            // which the decoded stream can exceed
            if (total != null && loaded > total) {
                total = null;
            }

            const now = performance.now();

            if (now - lastReport >= PROGRESS_INTERVAL) {
                lastReport = now;
                this.#reportProgress({
                    stage: "downloading",
                    loaded,
                    total,
                });
            }
        }
        this.#reportProgress({
            stage: "downloading",
            loaded,
            total: total ?? loaded,
        });

        const data = new Uint8Array(loaded);
        let offset = 0;

        for (const chunk of chunks) {
            data.set(chunk, offset);
            offset += chunk.byteLength;
        }
        return data;
    }

    public hasId(moduleId: number): moduleId is TModuleId {
        return this.#bundle.has_id(moduleId);
    }

    public getFormattedSource(moduleId: TModuleId): string {
        return this.#bundle.get_module_text(moduleId);
    }

    async generateDefinitions(moduleId: TModuleId, position: Monaco.IPosition) {
        const mPos = new MonacoPosition(position.lineNumber, position.column);
        const rawDefs = await this.#bundle.provide_definition(moduleId, mPos);

        return rawDefs.map(convertModuleLocation);
    }

    async generateReferences(moduleId: TModuleId, position: Monaco.IPosition) {
        const mPos = new MonacoPosition(position.lineNumber, position.column);
        const rawRefs = await this.#bundle.provide_references(moduleId, mPos);

        return rawRefs.map(convertModuleLocation);
    }

    async generateHover(moduleId: TModuleId, position: Monaco.IPosition) {
        const mPos = new MonacoPosition(position.lineNumber, position.column);
        const hoverInfo = await this.#bundle.provide_hover(moduleId, mPos);

        return hoverInfo && convertHoverInfo(hoverInfo);
    }

    getAllModuleIds(): Uint32Array {
        const ids = this.#bundle.get_id_list();

        return comlink.transfer(ids, [ids.buffer]);
    }

    public searchModules(query: string, regex: boolean): BundleSearchResults {
        const results = this.#bundle.search_modules(query, regex);
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

    public getExperiments(): ExperimentInfo[] {
        return this.#bundle.get_experiments();
    }

    public async getIcons(): Promise<IIcon[]> {
        const icons = await this.#bundle.get_icons();

        return icons.map((icon) => ({
            definedIn: icon.defined_in as TModuleId,
            pos: convertRange(icon.pos),
            name: icon.name,
            svg: icon.svg,
        }) satisfies IIcon);
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
