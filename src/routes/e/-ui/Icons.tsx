import { useQuery } from "@tanstack/react-query";

import { useModuleViewerStore } from "../-data";
import type { IIcon } from "../-data/worker/sharedWorker";

function useBuildService() {
    return useModuleViewerStore((s) => s._buildService);
}

function iconKey(icon: IIcon) {
    return `${icon.definedIn}:${icon.pos.startColumn}:${icon.pos.startLineNumber}:${icon.pos.endColumn}:${icon.pos.endLineNumber}`;
}
export function IconBrowser() {
    const buildHash = useModuleViewerStore((s) => s.buildHash);
    const service = useBuildService();

    const { data: icons, status, error } = useQuery({
        queryKey: ["icons", buildHash],
        queryFn() {
            return service.getIcons();
        },
    });

    if (status === "pending") {
        return <div>Loading icons...</div>;
    } else if (status === "success") {
        return icons.map((icon) => {
            return <div key={iconKey(icon)}>icon {icon.name ?? "Unknown"}</div>;
        });
    }
    return <div>Error loading icons: {String(error)}</div>;
}
