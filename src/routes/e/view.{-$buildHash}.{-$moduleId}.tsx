import { createFileRoute, redirect } from "@tanstack/react-router";

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
        if (import.meta.env.SSR) {
            return;
        }

        const { ModuleViewerStore } = await import("./-data");

        ModuleViewerStore.getState().init(buildHash);
        ModuleViewerStore.setState({
            selectedModule: moduleId,
        });
        if (moduleId != null) {
            // preload code
            await ModuleViewerStore.getState().getModuleCode(moduleId);
        }
    },
    ssr: false,
});

function ExplorerWrapper() {
    return <Explorer />;
}

