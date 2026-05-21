import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/layout/Box";
import { HorizontalLine } from "@/components/Lines";
import { TextLink } from "@/components/Links";
import { Text } from "@/components/Text";
import { Tooltip } from "@/components/Tooltip";
import { EM_DASH } from "@/utils/constants";
import type { TBundleHash } from "@/utils/types";
import { useQuery } from "@tanstack/react-query";

import type { GetBuildsFn, Meta } from "./-worker";

import * as comlink from "comlink";
import { ExternalLinkIcon } from "lucide-react";
import prodWorkerUrl from "omt:./-worker";
import { useMemo } from "react";

let workerUrl: string;

// OMT doesn't work in dev mode
if (!import.meta.env.SSR && import.meta.env.DEV) {
    ({ default: workerUrl } = await import("./-worker?sharedworker&url"));
} else {
    workerUrl = prodWorkerUrl;
}

interface BundleItemProps {
    bundleMeta: Meta;
}

declare global {
    interface WorkerOptions {
        /**
         * ONLY FOR SHARED WORKERS
         * 
         * A boolean indicating whether the shared worker is allowed to remain alive for a short period after all pages using it have been navigated away from or closed.
         *
         * This is provided to allow work to be done after the user navigates away from the page, such as writing state information to storage, or sending analytics data back to servers. The exact time that the worker is kept alive depends on the browser, and could be anywhere between 10 seconds and 5 minutes (Chrome uses 30 seconds).
         *
         * For more information see {@link https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers#shared_worker_lifetime|Shared worker lifetime} in Using web workers.
         */
        extendedLifetime?: boolean;
    }
}

async function getBuilds() {
    // this is flakey when used as a shared worker
    const worker = new SharedWorker(workerUrl, {
        type: "module",
        name: "fetch-builds-worker",
        extendedLifetime: !import.meta.env.DEV,
    });

    const getBuildsFn = comlink.wrap<GetBuildsFn>(worker.port);
    const ret = await getBuildsFn();

    return ret;
}

const SEPARATOR = (
    <Text
        tag="span"
    >
        {" | "}
    </Text>
);

function BundleItem({ bundleMeta }: BundleItemProps) {
    return (
        <li className="flex justify-between">
            <div>
                <Text tag="span">
                    Build Number {EM_DASH} {bundleMeta.build_number}
                </Text>
                {SEPARATOR}
                <Tooltip
                    text={bundleMeta.build_hash}
                >
                    <Text
                        tag="span"
                        className="underline decoration-dashed underline-offset-2"
                    >
                        Build Hash
                    </Text>
                </Tooltip>
                {SEPARATOR}
                <Text tag="span">
                    First Seen {EM_DASH} {new Date(Number(bundleMeta.first_seen)).toLocaleString()}
                </Text>
            </div>
            <div>
                <TextLink
                    to="/e/view/{-$buildHash}/{-$moduleId}"
                    params={{
                        buildHash: bundleMeta.build_hash as TBundleHash,
                        moduleId: null,
                    }}
                    color="primary"
                >
                    Open Bundle <ExternalLinkIcon className="inline" />
                </TextLink>
            </div>
        </li>
    );
}

export function BundleSelector() {
    const { status, data } = useQuery({
        queryKey: ["getAvailableBundles"],
        queryFn() {
            return getBuilds();
        },
    });

    const sortedBundles = useMemo(() => data?.toSorted(({ first_seen: fa }, { first_seen: fb }) => {
        if (fa === fb) {
            return 0;
        }
        if (fb > fa) {
            return 1;
        }
        return -1;
    }), [data]);

    return (
        <>
            <Boilerplate />
            <div className="flex justify-center pt-4">
                <Box className="min-w-1/2">
                    <Text
                        size="xl"
                        center
                    >
                        Select a build
                    </Text>
                    <HorizontalLine
                        color="white-700"
                    />
                    {status === "pending" && (
                        <Text
                            size="lg"
                            color="accent"
                            center
                        >
                            Loading...
                        </Text>
                    )}
                    {status === "error" && (
                        <Text
                            size="lg"
                            color="error"
                            center
                        >
                            An error occurred while loading the bundles.
                        </Text>
                    )}
                    {status === "success" && (
                        <ul>
                            {sortedBundles!.length === 0 && (
                                <Text
                                    color="error"
                                    size="lg"
                                    center
                                >
                                    No Bundles Available.
                                    <p />
                                    This is an error. Please report this.
                                </Text>
                            )}
                            {sortedBundles!.map((bundleMeta) => {
                                return (
                                    <BundleItem
                                        key={bundleMeta.build_hash}
                                        bundleMeta={bundleMeta}
                                    />
                                );
                            })}
                        </ul>
                    )}
                </Box>
            </div>
        </>
    );
}
