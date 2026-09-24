import { unavailableImport } from "@/utils/error";
import { createFileRoute } from "@tanstack/react-router";

const ui = import.meta.env.SSR ? unavailableImport("./-ui") : await import("./-ui");

export const Route = createFileRoute("/_/e/")({
    component: RouteComponent,
    ssr: false,
    staticData: {
        description: "Browse the webpack modules of any archived Discord build.",
        pageTitle: "Discord Bundle Explorer",
    },
});

function RouteComponent() {
    return <ui.BundleSelector />;
}
