import { assetUrl, Channels } from "./constants";

export function fetchAsset(channel: Channels, assetPath: string, opts?: RequestInit): Promise<Response> {
    if (assetPath.startsWith("/")) {
        assetPath = assetPath.slice(1);
    }
    return fetch(`${assetUrl[channel]}${assetPath}`, opts);
}
