import { Box } from "@/components/layout/Box";
import { Text } from "@/components/Text";
import { unavailableImport } from "@/utils/error";
import { TBundleHash } from "@/utils/types";
import { createFileRoute, redirect } from "@tanstack/react-router";

import { type DiffSide, useDiffLoadStore } from "./-data/loadProgress";
import { BundleLoadProgressBar } from "./-ui/LoadProgress";

import { LoaderCircle } from "lucide-react";
import z from "zod";

const ui = import.meta.env.SSR ? unavailableImport("./-ui/ExperimentDiff") : await import("./-ui/ExperimentDiff");

const diffParamsSchema = z.object({
    hashA: TBundleHash.catch("" as TBundleHash),
    hashB: TBundleHash.catch("" as TBundleHash),
});

async function loadExperiments(hash: TBundleHash, side: DiffSide) {
    const { getBuildService } = await import("./-data/worker/api");
    const { setProgress } = useDiffLoadStore.getState();

    setProgress(side, null);

    const buildService = await getBuildService(hash, (progress) => {
        setProgress(side, progress);
    });

    return await buildService.getExperiments();
}

export const Route = createFileRoute("/e/diff/$hashA/$hashB")({
    component: ExperimentDiffWrapper,
    pendingComponent: DiffLoading,
    // loading two full bundles can take a long time
    pendingMs: 100,
    params: {
        parse(raw) {
            const result = diffParamsSchema.parse(raw);

            if (!result.hashA || !result.hashB) {
                // oxlint-disable-next-line typescript/only-throw-error
                throw redirect({
                    to: "/e",
                });
            }

            return result;
        },
    },
    async loader({ params: { hashA, hashB } }) {
        const [before, after] = await Promise.all([
            loadExperiments(hashA, "a"),
            loadExperiments(hashB, "b"),
        ]);

        return {
            before,
            after,
        };
    },
    ssr: false,
});

function ExperimentDiffWrapper() {
    return <ui.ExperimentDiff />;
}

interface SideProgressProps {
    side: DiffSide;
    hash: TBundleHash;
}

function SideProgress({ side, hash }: SideProgressProps) {
    const progress = useDiffLoadStore((s) => s.progress[side]);

    return (
        <div className="flex flex-col items-center gap-2">
            <BundleLoadProgressBar progress={progress} />
            <code
                className="text-xs text-fg-700"
                title={hash}
            >
                {hash}
            </code>
        </div>
    );
}

function DiffLoading() {
    const { hashA, hashB } = Route.useParams();

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
                    Loading builds
                </Text>
                <SideProgress
                    side="a"
                    hash={hashA}
                />
                <SideProgress
                    side="b"
                    hash={hashB}
                />
            </Box>
        </div>
    );
}
