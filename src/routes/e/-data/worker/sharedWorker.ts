/// <reference lib="webworker" />

import { assert } from "@/utils/error";
import type { TBundleHash } from "@/utils/types";

import * as comlink from "comlink";

export interface IBuildService {
    readonly bundleHash: TBundleHash;
    add(a: number, b: number): number;
}

const self = globalThis as any as SharedWorkerGlobalScope;

class BuildService implements IBuildService {
    public bundleHash!: TBundleHash;
    #build: any = null;

    public init(hash: TBundleHash) {
        if (this.bundleHash) {
            assert(this.bundleHash === hash, "Worker already initialized with a different bundle hash");
        }
        if (this.#build == null) {
            await this.#downloadBuild();
        }
        this.bundleHash = hash;
    }

    #downloadBuild() {
        
    }

    public add(a: number, b: number): number {
        console.log(`Adding ${a} and ${b} in worker for bundle ${this.bundleHash}`);
        return a + b + 1;
    }
}

// use a type alias to avoid including BuildService in auto imports
export type RawBuildService = BuildService;

self.onconnect = ({ ports: [port] }) => {
    comlink.expose(new BuildService(), port);
};
