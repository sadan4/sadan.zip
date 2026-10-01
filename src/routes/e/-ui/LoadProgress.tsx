import { Box } from "@/components/layout/Box";
import { Text } from "@/components/Text";
import cn from "@/utils/cn";
import { MiB } from "@/utils/constants";

import { type BundleLoadProgress, useBundleLoadStore } from "../-data/loadProgress";

import { LoaderCircle } from "lucide-react";

// imported by route loading screens, must not pull in monaco

function formatMiB(bytes: number) {
    return `${(bytes / MiB).toFixed(1)} MiB`;
}

/**
 * @returns progress in [0, 1] or null
 */
function getProgressFraction(progress: BundleLoadProgress | null): number | null {
    if (progress?.stage !== "downloading" || progress.total == null) {
        return null;
    }
    return Math.min(progress.loaded / progress.total, 1);
}

function getProgressLabel(progress: BundleLoadProgress | null): string {
    switch (progress?.stage) {
        case undefined:
            return "Starting...";
        case "downloading":
            return progress.total == null
                ? `Downloading ${formatMiB(progress.loaded)}`
                : `Downloading ${formatMiB(progress.loaded)} / ${formatMiB(progress.total)}`;
        case "processing":
            return "Processing bundle...";
    }
}

export interface BundleLoadProgressBarProps {
    progress: BundleLoadProgress | null;
}

export function BundleLoadProgressBar({ progress }: BundleLoadProgressBarProps) {
    const fraction = getProgressFraction(progress);

    return (
        <div className="w-72 max-w-full">
            <div
                role="progressbar"
                aria-label="Bundle loading progress"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={fraction == null ? undefined : Math.round(fraction * 100)}
                className="h-2 w-full overflow-hidden rounded-full bg-bg-300"
            >
                <div
                    className={cn(
                        "h-full rounded-full bg-primary-400",
                        fraction == null ? "w-full animate-pulse" : "transition-[width] duration-100 ease-linear",
                    )}
                    style={fraction == null ? undefined : { width: `${fraction * 100}%` }}
                />
            </div>
            <div className="mt-2 flex justify-between gap-3 text-xs text-fg-700">
                <span aria-live="polite">{getProgressLabel(progress)}</span>
                {fraction != null && <span>{Math.round(fraction * 100)}%</span>}
            </div>
        </div>
    );
}

export interface BundleLoadingScreenProps {
    buildHash: string;
}

/**
 * full page loading screen for the build `ModuleViewerStore.init` is loading
 */
export function BundleLoadingScreen({ buildHash }: BundleLoadingScreenProps) {
    const progress = useBundleLoadStore((s) => s.progress);

    return (
        <div className="flex min-h-[calc(100dvh-8rem)] items-center justify-center px-4">
            <Box
                className="flex flex-col items-center gap-4 px-8 py-10"
                role="status"
            >
                <LoaderCircle
                    className="size-10 animate-spin text-primary-400"
                    aria-hidden="true"
                />
                <Text
                    size="xl"
                    weight="semiBold"
                >
                    Loading build
                </Text>
                <BundleLoadProgressBar progress={progress} />
                <code
                    className="text-xs text-fg-700"
                    title={buildHash}
                >
                    {buildHash}
                </code>
            </Box>
        </div>
    );
}
