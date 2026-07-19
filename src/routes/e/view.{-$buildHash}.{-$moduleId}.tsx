import { unavailableImport } from "@/utils/error";
import { TBundleHash } from "@/utils/types";
import { createFileRoute, redirect } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";

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

