import { unavailableImport } from "@/utils/error";
import { createFileRoute } from "@tanstack/react-router";

const ui = import.meta.env.SSR ? unavailableImport("@/components/ASTViewer") : await import("@/components/ASTViewer/index.tsrx");

export const Route = createFileRoute("/ast-viewer/")({
    component: RouteComponent,
    ssr: false,
});

function RouteComponent() {
    return <ui.ASTViewer />;
}
