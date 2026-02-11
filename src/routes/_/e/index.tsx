import { unavailableImport } from "@/utils/error";
import { createFileRoute } from "@tanstack/react-router";

const ui = import.meta.env.SSR ? unavailableImport<never>("./-ui") : require("./-ui") as typeof import("./-ui");

export const Route = createFileRoute("/_/e/")({
    component: RouteComponent,
    ssr: false,
});

function RouteComponent() {
    return <ui.BundleSelector />;
}
