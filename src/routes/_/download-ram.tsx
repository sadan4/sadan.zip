import { Boilerplate } from "@/components/Boilerplate";
import { RAMDownloader } from "@/components/RAMDownloader/RAMDownloader";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_/download-ram")({
    component: RAMDownloaderPage,
    staticData: {
        description: "Upgrade your PC instantly with our patented cloud RAM technology.",
        pageTitle: "Download More RAM",
    },
});

function RAMDownloaderPage() {
    return (
        <div className="flex min-h-[60vh] flex-col items-center justify-center p-4">
            <Boilerplate />
            <RAMDownloader />
        </div>
    );
}
