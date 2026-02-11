import { unavailableImport } from "@/utils/error";
import { createFileRoute } from "@tanstack/react-router";

const ui = import.meta.env.SSR ? unavailableImport("@/components/ASTViewer") : require("@/components/ASTViewer") as typeof import("@/components/ASTViewer");

export const Route = createFileRoute("/ast-viewer/")({
    component: RouteComponent,
    ssr: false,
});

function RouteComponent() {
    return <ui.ASTViewer />;
}
