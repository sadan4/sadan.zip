import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { Box } from "@/components/layout/Box";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { Link } from "@/components/Links";
import { Text } from "@/components/Text";
import { useToaster } from "@/hooks/toaster";
import { copyWithNotify } from "@/utils/clipboard";
import cn from "@/utils/cn";
import type { TBundleHash } from "@/utils/types";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import type { Channel, GetBuildsFn, Meta } from "./-worker";

import * as comlink from "comlink";
import { ArrowRight, Check, Clock3, Hash } from "lucide-react";
import prodWorkerUrl from "omt:./-worker";
import { useMemo, useState } from "react";

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
    /**
     * set when picking builds to compare, the item toggles selection instead of opening the build
     */
    compare?: {
        selected: boolean;
        onToggle(): void;
    };
}

/**
 * two builds are compared at once
 */
const COMPARE_COUNT = 2;

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

function BundleItem({ bundleMeta, latestTags, compare }: BundleItemProps) {
    // a "Latest <channel>" tag replaces the plain channel pill
    const pills: PillKind[] = [
        ...latestTags,
        ...bundleMeta.channels.filter((channel) => !latestTags.includes(`latest-${channel}`)),
    ];

    const ToastStore = useToaster();
    const buildDate = new Date(Number(bundleMeta.first_seen));
    const itemClass = "group flex h-full w-full items-center gap-4 rounded-md border border-fg-700/60 bg-bg-200 px-4 py-3 text-left transition-colors hover:border-primary-400/70 hover:bg-bg-300 focus-visible:border-primary-400 focus-visible:bg-bg-300";

    const content = (
        <>
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
                    <span className="flex min-w-0 items-center gap-1 text-xs text-fg-700">
                        <Clock3
                            className="size-3 shrink-0"
                            aria-hidden="true"
                        />
                        <span title={buildDate.toLocaleString(undefined, strftimeOptions)}>
                            {buildDate.toLocaleString()}
                        </span>
                    </span>
                    <Clickable
                        tag="span"
                        className="flex items-center gap-1 text-xs text-fg-700"
                        onClick={async (e) => {
                            e.stopPropagation();
                            e.preventDefault();
                            await copyWithNotify(bundleMeta.build_hash, ToastStore.getState());
                        }}
                        title={bundleMeta.build_hash}
                    >
                        <Hash
                            className="size-3 shrink-0"
                            aria-hidden="true"
                        />
                        <code>{bundleMeta.build_hash.slice(0, 9)}</code>
                    </Clickable>
                </div>
            </div>
            {compare
                ? (
                    <span
                        className={cn(
                            "flex size-5 shrink-0 items-center justify-center rounded-sm border",
                            compare.selected ? "border-primary-400 bg-primary-400 text-bg-100" : "border-fg-700",
                        )}
                        aria-hidden="true"
                    >
                        {compare.selected && <Check className="size-4" />}
                    </span>
                )
                : (
                    <span className="flex shrink-0 items-center gap-1 text-sm text-primary-400">
                        <span className="hidden sm:inline">Open</span>
                        <ArrowRight
                            className="size-4 transition-transform group-hover:translate-x-0.5"
                            aria-hidden="true"
                        />
                    </span>
                )}
        </>
    );

    return (
        <li>
            {compare
                ? (
                    <button
                        type="button"
                        aria-pressed={compare.selected}
                        aria-label={`Select build ${bundleMeta.build_number} to compare`}
                        className={cn(itemClass, compare.selected && "border-primary-400 bg-bg-300")}
                        onClick={compare.onToggle}
                    >
                        {content}
                    </button>
                )
                : (
                    <Link
                        to="/e/view/{-$buildHash}/{-$moduleId}"
                        params={{
                            buildHash: bundleMeta.build_hash as TBundleHash,
                            moduleId: null,
                        }}
                        preload={false}
                        aria-label={`Open build ${bundleMeta.build_number}`}
                        className={itemClass}
                    >
                        {content}
                    </Link>
                )}
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

    const navigate = useNavigate();
    const [compareMode, setCompareMode] = useState(false);
    /**
     * build hashes picked for comparison, in the order they were picked
     */
    const [compareSelection, setCompareSelection] = useState<readonly string[]>([]);

    function toggleCompareSelection(hash: string) {
        setCompareSelection((prev) => {
            if (prev.includes(hash)) {
                return prev.filter((h) => h !== hash);
            }
            // picking another build replaces the oldest pick
            return [...prev, hash].slice(-COMPARE_COUNT);
        });
    }

    async function compareSelected() {
        // older build on the left
        const [hashA, hashB] = compareSelection
            .map((hash) => sortedBundles!.find(({ build_hash }) => build_hash === hash)!)
            .toSorted((a, b) => Number(a.first_seen - b.first_seen))
            .map(({ build_hash }) => build_hash as TBundleHash);

        await navigate({
            to: "/e/diff/$hashA/$hashB",
            params: {
                hashA,
                hashB,
            },
        });
    }

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
                            <div className="flex flex-wrap items-center gap-3">
                                <ChannelFilter
                                    enabled={enabledChannels}
                                    onToggle={toggleChannel}
                                />
                                <Button
                                    size="sm"
                                    colorType={compareMode ? "filled" : "outline"}
                                    aria-pressed={compareMode}
                                    onClick={() => {
                                        setCompareMode((prev) => !prev);
                                        setCompareSelection([]);
                                    }}
                                >
                                    Compare builds
                                </Button>
                                {compareMode && (
                                    <Button
                                        size="sm"
                                        disabled={compareSelection.length !== COMPARE_COUNT}
                                        onClick={compareSelected}
                                    >
                                        Compare ({compareSelection.length}/{COMPARE_COUNT})
                                    </Button>
                                )}
                                <Text
                                    size="sm"
                                    color="white-700"
                                >
                                    {filteredBundles!.length === sortedBundles!.length
                                        ? `${sortedBundles!.length} available`
                                        : `${filteredBundles!.length} of ${sortedBundles!.length} shown`}
                                </Text>
                            </div>
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
                            <ul className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
                                {sortedBundles!.length === 0 && (
                                    <Text
                                        color="error"
                                        size="lg"
                                        center
                                        className="col-span-full"
                                    >
                                        No Bundles Available.
                                        <p />
                                        This is an error. Please report this.
                                    </Text>
                                )}
                                {sortedBundles!.length > 0 && filteredBundles!.length === 0 && (
                                    <Text
                                        color="white-700"
                                        size="lg"
                                        center
                                        className="col-span-full py-10"
                                    >
                                        No builds match the selected channels.
                                    </Text>
                                )}
                                {filteredBundles!.map((bundleMeta) => {
                                    return (
                                        <BundleItem
                                            key={bundleMeta.build_hash}
                                            bundleMeta={bundleMeta}
                                            latestTags={latestTagsByHash.get(bundleMeta.build_hash) ?? []}
                                            compare={compareMode
                                                ? {
                                                    selected: compareSelection.includes(bundleMeta.build_hash),
                                                    onToggle() {
                                                        toggleCompareSelection(bundleMeta.build_hash);
                                                    },
                                                }
                                                : undefined}
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
