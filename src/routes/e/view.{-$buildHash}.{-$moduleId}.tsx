import { createFileRoute, redirect } from "@tanstack/react-router";

import { useModuleViewerStore } from "./-data";
import { Explorer } from "./-ui";

import z from "zod";

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
        useModuleViewerStore.getState().init(buildHash);
        useModuleViewerStore.setState({
            selectedModule: moduleId,
        });
        if (moduleId != null) {
            // preload code
            await useModuleViewerStore.getState().getModuleCode(moduleId);
        }
    },
    ssr: false,
});

function ExplorerWrapper() {
    return <Explorer />;
}

