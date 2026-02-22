import { unavailableImport } from "@/utils/error";
import { createFileRoute, redirect } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";

import { registerLSPHandlers } from "./-lsp";
import { TBundleHash, TModuleId } from "../../../server/types";

import z from "zod";

const data = import.meta.env.SSR ? unavailableImport("./-data") : await import("./-data");
const ui = import.meta.env.SSR ? unavailableImport("./-ui") : await import("./-ui");

const viewBundleParamsSchema = z.object({
    buildHash: TBundleHash.catch("" as TBundleHash),
    moduleId: TModuleId
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
                throw redirect({
                    to: "/e",
                });
            }

            return result;
        },
    },
    async loader({ params: { buildHash, moduleId } }) {
        await registerLSPHandlers();
        data.ModuleViewerStore.getState().init(buildHash);
        if (moduleId != null) {
            // preload code
            await data.ModuleViewerStore.getState().getModuleCode(moduleId);
        }
    },
    validateSearch: zodValidator(searchParamsSchema),
    ssr: false,
});

function ExplorerWrapper() {
    return <ui.Explorer />;
}

