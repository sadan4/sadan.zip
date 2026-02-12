import { assetUrl, Channels } from "./constants";

export function fetchAsset(channel: Channels, assetPath: string, opts?: RequestInit): Promise<Response> {
    if (assetPath.startsWith("/")) {
        assetPath = assetPath.slice(1);
    }
    return fetch(`${assetUrl[channel]}${assetPath}`, opts);
}

export function keys<T extends Record<PropertyKey, any>>(obj: T): (keyof T)[] {
    return Object.keys(obj) as (keyof T)[];
}

export function values<T extends Record<PropertyKey, any>>(obj: T): T[keyof T][] {
    return Object.values(obj) as T[keyof T][];
}

export function entries<T extends Record<PropertyKey, any>>(obj: T): [keyof T, T[keyof T]][] {
    return Object.entries(obj) as [keyof T, T[keyof T]][];
}
