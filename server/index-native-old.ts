import { unreachable } from "@/utils/error";

import { Channels } from "./constants";
import native from "./native";
import type { ParserWorkerData } from "./index-native";
import type { TBundleHash } from "./types";

import { Worker } from "node:worker_threads";

function nativeChannelToJsChannel(nc: native.Channel): Channels {
    switch (nc) {
        case native.Channel.Stable:
            return Channels.STABLE;
        case native.Channel.Canary:
            return Channels.CANARY;
        default:
            unreachable();
    }
}

function handleBuild(data: native.HandleBuildOpts) {
    new Worker(new URL("./native-parserWorker", import.meta.url), {
        name: "Native Parser Worker",
        workerData: {
            buildHash: data.buildHash as TBundleHash,
            html: data.html,
            channel: nativeChannelToJsChannel(data.channel),
        } satisfies ParserWorkerData,
    });
}

native.start(handleBuild);
