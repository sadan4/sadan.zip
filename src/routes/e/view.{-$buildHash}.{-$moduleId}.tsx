import { Box } from "@/components/layout/Box";
import { Text } from "@/components/Text";
import { unavailableImport } from "@/utils/error";
import { TBundleHash } from "@/utils/types";
import { createFileRoute, redirect } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";

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

function BundleLoading() {
    const { buildHash } = Route.useParams();

    return (
        <div className="flex min-h-[calc(100dvh-8rem)] items-center justify-center px-4">
            <Box
                className="flex flex-col items-center gap-4 px-8 py-10"
                role="status"
                aria-live="polite"
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
                <Text
                    size="sm"
                    color="white-700"
                    center
                >
                    Loading the full bundle. This can take a few seconds.
                </Text>
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
