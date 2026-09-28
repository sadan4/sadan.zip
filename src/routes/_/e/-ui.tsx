import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/layout/Box";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { Link } from "@/components/Links";
import { Text } from "@/components/Text";
import type { TBundleHash } from "@/utils/types";
import { useQuery } from "@tanstack/react-query";

import type { GetBuildsFn, Meta } from "./-worker";

import * as comlink from "comlink";
import { ArrowRight, Clock3, Hash } from "lucide-react";
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

let getBuildsFn: comlink.Remote<GetBuildsFn> | null = null;

async function getBuilds() {
    if (!getBuildsFn) {
        const worker = new SharedWorker(workerUrl, {
            type: "module",
            name: "fetch-builds-worker",
            extendedLifetime: !import.meta.env.DEV,
        });

        getBuildsFn = comlink.wrap<GetBuildsFn>(worker.port);
    }

    const ret = await getBuildsFn();

    return ret;
}

function BundleItem({ bundleMeta }: BundleItemProps) {
    return (
        <li>
            <Link
                to="/e/view/{-$buildHash}/{-$moduleId}"
                params={{
                    buildHash: bundleMeta.build_hash as TBundleHash,
                    moduleId: null,
                }}
                preload={false}
                aria-label={`Open build ${bundleMeta.build_number}`}
                className="group flex items-center gap-4 rounded-md border border-fg-700/60 bg-bg-200 px-4 py-3 transition-colors hover:border-primary-400/70 hover:bg-bg-300 focus-visible:border-primary-400 focus-visible:bg-bg-300"
            >
                <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-3">
                        <Text
                            tag="span"
                            size="md"
                            weight="semiBold"
                            color="primary"
                        >
                            Build {bundleMeta.build_number}
                        </Text>
                        <span
                            className="flex items-center gap-1 text-xs text-fg-700"
                            title={bundleMeta.build_hash}
                        >
                            <Hash
                                className="size-3 shrink-0"
                                aria-hidden="true"
                            />
                            <code>{bundleMeta.build_hash.slice(0, 9)}</code>
                        </span>
                    </div>
                    <div className="mt-1.5">
                        <span className="flex min-w-0 items-center gap-1 text-xs text-fg-700">
                            <Clock3
                                className="size-3 shrink-0"
                                aria-hidden="true"
                            />
                            <span>{new Date(Number(bundleMeta.first_seen)).toLocaleString()}</span>
                        </span>
                    </div>
                </div>
                <span className="flex shrink-0 items-center gap-1 text-sm text-primary-400">
                    <span className="hidden sm:inline">Open</span>
                    <ArrowRight
                        className="size-4 transition-transform group-hover:translate-x-0.5"
                        aria-hidden="true"
                    />
                </span>
            </Link>
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
            <div className="mx-auto w-full max-w-6xl px-4 pt-8">
                <Box className="p-4 sm:p-6">
                    <div className="flex flex-wrap items-end justify-between gap-3">
                        <div>
                            <Text
                                size="xl"
                                weight="semiBold"
                            >
                                Choose a build
                            </Text>
                        </div>
                        {status === "success" && (
                            <Text
                                size="sm"
                                color="white-700"
                            >
                                {sortedBundles!.length} available
                            </Text>
                        )}
                    </div>
                    {status === "pending" && (
                        <Text
                            size="lg"
                            color="accent"
                            center
                            className="py-10"
                        >
                            Loading...
                        </Text>
                    )}
                    {status === "error" && (
                        <Text
                            size="lg"
                            color="error"
                            center
                            className="py-10"
                        >
                            An error occurred while loading the bundles.
                        </Text>
                    )}
                    {status === "success" && (
                        <ScrollArea className="mt-4 max-h-[calc(100dvh-11.25rem)]">
                            <ul className="space-y-2">
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
                        </ScrollArea>
                    )}
                </Box>
            </div>
        </>
    );
}
