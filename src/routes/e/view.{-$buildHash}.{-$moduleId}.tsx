import { unavailableImport } from "@/utils/error";
import { createFileRoute, redirect } from "@tanstack/react-router";

import z from "zod";

const data = import.meta.env.SSR ? unavailableImport("./-data") : await import("./-data");
const ui = import.meta.env.SSR ? unavailableImport("./-ui") : await import("./-ui");

const viewBundleParamsSchema = z.object({
    buildHash: z.string().catch(""),
    moduleId: z.string()
        .nullable()
        .catch(null),
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
        data.ModuleViewerStore.getState().init(buildHash);
        data.ModuleViewerStore.setState({
            selectedModule: moduleId,
        });
        if (moduleId != null) {
            // preload code
            await data.ModuleViewerStore.getState().getModuleCode(moduleId);
        }
    },
    ssr: false,
});

function ExplorerWrapper() {
    return <ui.Explorer />;
}

