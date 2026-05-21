/// <reference lib="webworker" />

import { assert } from "@/utils/error";
import type { Monaco } from "@/utils/monaco";
import type { TBundleHash, TModuleId } from "@/utils/types";
import { Bundle, default as initWasm, get_bundle, HoverInfo as RawHoverInfo, ModuleLocation as RawModuleLocation, MonacoPosition, MonacoRange } from "@sadan4/libsadancore";

import * as comlink from "comlink";

export interface ModuleLocation {
    range: Monaco.IRange;
    id: TModuleId;
}

export interface HoverInfo {
    content: string;
    range: Monaco.IRange;
}

export interface IBuildService {
    hasId(moduleId: number): moduleId is TModuleId;
    getFormattedSource(moduleId: TModuleId): string;
    generateDefinitions(moduleId: TModuleId, position: Monaco.IPosition): ModuleLocation[];
    generateReferences(moduleId: TModuleId, position: Monaco.IPosition): ModuleLocation[];
    generateHover(moduleId: TModuleId, position: Monaco.IPosition): HoverInfo | undefined;
    getAllModuleIds(): Uint32Array;
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

function convertHoverInfo({ content, range }: RawHoverInfo): HoverInfo {
    return {
        content,
        range: convertRange(range),
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
}

// use a type alias to avoid including BuildService in auto imports
export type RawBuildService = BuildService;
{
    const service = new BuildService();

    self.onconnect = ({ ports: [port] }) => {
        comlink.expose(service, port);
    };
}
