import { Box } from "@/components/layout/Box";
import { Text } from "@/components/Text";
import cn from "@/utils/cn";
import { MiB } from "@/utils/constants";
import { unavailableImport } from "@/utils/error";
import { TBundleHash } from "@/utils/types";
import { createFileRoute, redirect } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";

import { type BundleLoadProgress, useBundleLoadStore } from "./-data/loadProgress";

import { LoaderCircle } from "lucide-react";
import z from "zod";

let data: typeof import("./-data") | null = import.meta.env.SSR ? unavailableImport("./-data") : null;
const ui = import.meta.env.SSR ? unavailableImport("./-ui") : await import("./-ui");

const viewBundleParamsSchema = z.object({
    buildHash: TBundleHash.catch("" as TBundleHash),
    moduleId: z.coerce.number()
        .nullable()
        .catch(null),
});

const searchParamsSchema = z.object({
    /**
     * range line start.
     * 1-based.
     */
    sl: z.number()
        .optional()
        .catch(undefined),
    /**
     * range character start.
     * 1-based.
     */
    sc: z.number()
        .optional()
        .catch(undefined),
    /**
     * range line end.
     * 1-based.
     */
    el: z.number()
        .optional()
        .catch(undefined),
    /**
     * range character end.
     * 1-based.
     */
    ec: z.number()
        .optional()
        .catch(undefined),
});

export const Route = createFileRoute("/e/view/{-$buildHash}/{-$moduleId}")({
    component: ExplorerWrapper,
    pendingComponent: BundleLoading,
    // show it quickly, loading a full bundle can take a long tim
    // ~20-30mb over network
    pendingMs: 100,
    params: {
        parse(raw) {
            const result = viewBundleParamsSchema.parse(raw);

            if (!result.buildHash) {
                // oxlint-disable-next-line typescript/only-throw-error
                throw redirect({
                    to: "/e",
                });
            }

            return result;
        },
    },
    beforeLoad(_) {
        // preload data and lsp modules
        if (!import.meta.env.SSR) {
            import("./-data").then((mod) => {
                data = mod;
            });
            import("./-lsp");
        }
    },
    async loader({ params: { buildHash } }) {
        if (!import.meta.env.SSR) {
            data ??= await import("./-data");

            const lsp = await import("./-lsp");

            await data.ModuleViewerStore.getState().init(buildHash);
            lsp.registerLSPHandlers();
        }
    },
    validateSearch: zodValidator(searchParamsSchema),
    ssr: false,
});

function ExplorerWrapper() {
    return <ui.Explorer />;
}

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

function BundleLoadProgressBar() {
    const progress = useBundleLoadStore((s) => s.progress);
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

function BundleLoading() {
    const { buildHash } = Route.useParams();

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
                <BundleLoadProgressBar />
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
