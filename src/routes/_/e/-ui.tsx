import { Boilerplate } from "@/components/Boilerplate";
import { Clickable } from "@/components/Clickable";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { ScrollAreaContext } from "@/components/layout/ScrollArea/context";
import { Link } from "@/components/Links";
import { Text } from "@/components/Text";
import { Tooltip } from "@/components/Tooltip";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { getNewestEntry, useIntersection } from "@/hooks/intersection";
import { useMediaQuery } from "@/hooks/mediaQuery";
import { useToaster } from "@/hooks/toaster";
import { copyWithNotify } from "@/utils/clipboard";
import cn from "@/utils/cn";
import { EM_DASH, NBSP } from "@/utils/constants";
import type { TBundleHash } from "@/utils/types";
import { useInfiniteQuery } from "@tanstack/react-query";

import type { Channel, GetBuildsPageFn, Meta } from "./-worker";

import * as comlink from "comlink";
import { ArrowRight, Clock3, Hash } from "lucide-react";
import prodWorkerUrl from "omt:./-worker";
import { use, useEffect, useMemo, useState } from "react";

let workerUrl: string;

// OMT doesn't work in dev mode
if (!import.meta.env.SSR && import.meta.env.DEV) {
    ({ default: workerUrl } = await import("./-worker?sharedworker&url"));
} else {
    workerUrl = prodWorkerUrl;
}

interface BundleItemProps {
    bundleMeta: Meta;
    latestTags: LatestTag[];
}

type LatestTag = "latest" | `latest-${Channel}`;

type PillKind = Channel | LatestTag;

const pillStyles: Record<PillKind, string> = {
    stable: "border-blurple/50 bg-blurple/15 text-[color-mix(in_oklab,var(--color-blurple)_60%,white)]",
    canary: "border-canary/50 bg-canary/15 text-canary",
    latest: "border-primary-400/40 bg-primary-400/10 text-primary-300",
    "latest-stable": "border-blurple bg-blurple/35 text-white",
    "latest-canary": "border-canary bg-canary/30 text-white",
};

const pillLabels: Record<PillKind, string> = {
    stable: "Stable",
    canary: "Canary",
    latest: "Latest",
    "latest-stable": "Latest Stable",
    "latest-canary": "Latest Canary",
};

const pillBaseClass = "rounded-full border px-2 py-0.5 text-xs leading-none font-medium";

function Pill({ kind }: { kind: PillKind; }) {
    return (
        <span className={cn(pillBaseClass, pillStyles[kind])}>
            {pillLabels[kind]}
        </span>
    );
}

const channels: Channel[] = ["stable", "canary"];

interface ChannelFilterProps {
    enabled: ReadonlySet<Channel>;
    onToggle(channel: Channel): void;
}

function ChannelFilter({ enabled, onToggle }: ChannelFilterProps) {
    return (
        <div
            role="group"
            aria-label="Filter by channel"
            className="flex items-center gap-1.5"
        >
            {channels.map((channel) => {
                const isEnabled = enabled.has(channel);

                return (
                    <button
                        key={channel}
                        type="button"
                        aria-pressed={isEnabled}
                        onClick={() => {
                            onToggle(channel);
                        }}
                        className={cn(
                            pillBaseClass,
                            "cursor-pointer transition-opacity",
                            isEnabled ? pillStyles[channel] : "border-fg-800 text-fg-800 line-through",
                        )}
                    >
                        {pillLabels[channel]}
                    </button>
                );
            })}
        </div>
    );
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

let getBuildsPageFn: comlink.Remote<GetBuildsPageFn> | null = null;

async function getBuildsPage(before: bigint | undefined) {
    if (!getBuildsPageFn) {
        const worker = new SharedWorker(workerUrl, {
            type: "module",
            name: "fetch-builds-worker",
            extendedLifetime: !import.meta.env.DEV,
        });

        getBuildsPageFn = comlink.wrap<GetBuildsPageFn>(worker.port);
    }

    return await getBuildsPageFn(before);
}

const strftimeOptions: Intl.DateTimeFormatOptions = {
    weekday: "long", // %A
    month: "long", // %B
    day: "2-digit", // %d
    year: "numeric", // %Y
    hour: "2-digit", // %I
    minute: "2-digit", // %M
    second: "2-digit", // %S
    hour12: true, // %p
    timeZoneName: "short", // %Z
};

function BundleItem({ bundleMeta, latestTags }: BundleItemProps) {
    // a "Latest <channel>" tag replaces the plain channel pill
    const pills: PillKind[] = [
        ...latestTags,
        ...bundleMeta.channels.filter((channel) => !latestTags.includes(`latest-${channel}`)),
    ];

    const ToastStore = useToaster();
    const buildDate = new Date(Number(bundleMeta.first_seen));

    return (
        <div>
            <Link
                to="/e/view/{-$buildHash}/{-$moduleId}"
                params={{
                    buildHash: bundleMeta.build_hash as TBundleHash,
                    moduleId: null,
                }}
                preload={false}
                aria-label={`Open build ${bundleMeta.build_number}`}
                className="group hover:border-primary-400/70 focus-visible:border-primary-400 focus-visible:bg-bg-300 flex h-full items-center gap-4 rounded-md border border-editorHoverWidget-border bg-editorHoverWidget-background px-4 py-3 transition-colors hover:bg-toolbar-hoverBackground"
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
                        {pills.length > 0 && (
                            <div className="flex flex-wrap gap-1.5">
                                {pills.map((kind) => (
                                    <Pill
                                        key={kind}
                                        kind={kind}
                                    />
                                ))}
                            </div>
                        )}
                    </div>
                    <div className="mt-1.5 flex items-center gap-3">
                        <Tooltip
                            text={buildDate.toLocaleString(undefined, strftimeOptions)}
                            position={TooltipPosition.BOTTOM}
                        >
                            <span className="text-fg-700 flex min-w-0 items-center gap-1 text-xs">
                                <Text color="description">
                                    <Clock3
                                        className="inline size-5 shrink-0"
                                        aria-hidden="true"
                                    />
                                    {NBSP}
                                    {buildDate.toLocaleString()}
                                </Text>
                            </span>
                        </Tooltip>
                        <Tooltip
                            position={TooltipPosition.BOTTOM}
                            text={`Click to copy ${EM_DASH} ${bundleMeta.build_hash}`}
                        >
                            <Clickable
                                tag="span"
                                className="text-fg-700 flex items-center gap-1 text-xs"
                                onClick={async (e) => {
                                    e.stopPropagation();
                                    e.preventDefault();
                                    await copyWithNotify(bundleMeta.build_hash, ToastStore.getState());
                                }}
                            >
                                <Text
                                    color="description"
                                    className="flex items-center justify-center"
                                >
                                    <Hash
                                        className="inline size-5 shrink-0"
                                        aria-hidden="true"
                                    />
                                    <code>{bundleMeta.build_hash.slice(0, 9)}</code>
                                </Text>
                            </Clickable>
                        </Tooltip>
                    </div>
                </div>
                <span className="text-primary-400 flex shrink-0 items-center gap-1 text-sm">
                    <span className="hidden sm:inline">Open</span>
                    <ArrowRight
                        className="size-4 transition-transform group-hover:translate-x-0.5"
                        aria-hidden="true"
                    />
                </span>
            </Link>
        </div>
    );
}

interface LoadMoreProps {
    canLoadMore: boolean;
    loadMore(): void;
}

/**
 * Footer of the build list that loads the next page while it's in view
 */
function LoadMore({ canLoadMore, loadMore }: LoadMoreProps) {
    const scrollAreaHandle = use(ScrollAreaContext);
    const [inView, setInView] = useState(false);

    const setIntersectionRef = useIntersection((entries) => {
        setInView(getNewestEntry(entries).isIntersecting);
    }, {
        rootRef: scrollAreaHandle.ref,
    });

    // re-checked after every page, as a page may add nothing that passes the channel filter
    useEffect(() => {
        if (inView && canLoadMore) {
            loadMore();
        }
    }, [inView, canLoadMore, loadMore]);

    return (
        <Text
            ref={setIntersectionRef}
            size="sm"
            center
            className="col-span-full py-4"
        >
            {canLoadMore ? "Loading more..." : "No more builds."}
        </Text>
    );
}

export function BundleSelector() {
    const { status, data, hasNextPage, isFetchingNextPage, fetchNextPage } = useInfiniteQuery({
        queryKey: ["getAvailableBundles"],
        queryFn({ pageParam }) {
            return getBuildsPage(pageParam);
        },
        initialPageParam: undefined as bigint | undefined,
        getNextPageParam: ({ nextCursor }) => nextCursor,
    });

    const large = useMediaQuery("(width >= 64rem)");
    const numCols = large ? 3 : 1;
    // pages come newest first
    const sortedBundles = useMemo(() => data?.pages.flatMap(({ builds }) => builds), [data]);
    const [enabledChannels, setEnabledChannels] = useState<ReadonlySet<Channel>>(() => new Set(channels));

    const latestTagsByHash = useMemo(() => {
        const tags = new Map<string, LatestTag[]>();

        function addTag(hash: string | undefined, tag: LatestTag) {
            if (hash == null) {
                return;
            }
            tags.set(hash, [...tags.get(hash) ?? [], tag]);
        }

        addTag(sortedBundles?.[0]?.build_hash, "latest");
        for (const channel of channels) {
            addTag(sortedBundles?.find((bundle) => bundle.channels.includes(channel))?.build_hash, `latest-${channel}`);
        }
        return tags;
    }, [sortedBundles]);

    // builds with no known channel are always shown
    // builds with a latest tag are pinned to the top, keeping newest-first order within each group
    const filteredBundles = useMemo(() => sortedBundles
        ?.filter(({ channels: buildChannels }) => {
            return buildChannels.length === 0 || buildChannels.some((channel) => enabledChannels.has(channel));
        })
        .toSorted((a, b) => {
            return Number(latestTagsByHash.has(b.build_hash)) - Number(latestTagsByHash.has(a.build_hash));
        }), [sortedBundles, enabledChannels, latestTagsByHash]);

    function toggleChannel(channel: Channel) {
        setEnabledChannels((prev) => {
            const next = new Set(prev);

            if (!next.delete(channel)) {
                next.add(channel);
            }
            return next;
        });
    }

    return (
        <>
            <Boilerplate />
            <div className="mx-auto w-full max-w-[90vw] px-4 pt-8">
                <div className="rounded-lg border-2 border-editorWidget-border bg-editor-background p-4 sm:p-6">
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
                            <div className="flex flex-wrap items-center gap-3">
                                <ChannelFilter
                                    enabled={enabledChannels}
                                    onToggle={toggleChannel}
                                />
                                <Text
                                    size="sm"
                                >
                                    {filteredBundles!.length === sortedBundles!.length
                                        ? `${sortedBundles!.length}${hasNextPage ? "+" : ""} available`
                                        : `${filteredBundles!.length} of ${sortedBundles!.length}${hasNextPage ? "+" : ""} shown`}
                                </Text>
                            </div>
                        )}
                    </div>
                    {status === "pending" && (
                        <Text
                            size="lg"
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
                        <BufferedScroller
                            items={filteredBundles!}
                            cols={numCols}
                            batchSize={50}
                            renderHeader={() => {
                                if (sortedBundles!.length === 0) {
                                    return (
                                        <Text
                                            size="lg"
                                            center
                                            className="col-span-full"
                                        >
                                            No Bundles Available.
                                            <p />
                                            This is an error. Please report this.
                                        </Text>
                                    );
                                }
                                if (sortedBundles!.length > 0 && filteredBundles!.length === 0) {
                                    return (
                                        <Text
                                            size="lg"
                                            center
                                            className="col-span-full py-10"
                                        >
                                            No builds match the selected channels.
                                        </Text>
                                    );
                                }
                                return null;
                            }}
                            renderFooter={() => {
                                if (sortedBundles!.length === 0) {
                                    return null;
                                }
                                return (
                                    <LoadMore
                                        canLoadMore={hasNextPage && !isFetchingNextPage}
                                        loadMore={fetchNextPage}
                                    />
                                );
                            }}
                            className="mt-4 max-h-[calc(100dvh-11.25rem)]"
                            renderItem={({ item: bundleMeta }) => {
                                return (
                                    <BundleItem
                                        key={bundleMeta.build_hash}
                                        bundleMeta={bundleMeta}
                                        latestTags={latestTagsByHash.get(bundleMeta.build_hash) ?? []}
                                    />
                                );
                            }}
                        />
                    )}
                </div>
            </div>
        </>
    );
}
