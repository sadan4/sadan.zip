import { createContext } from "react";

function handleF8(ev: KeyboardEvent) {
    if (ev.key === "F8" && !(ev.ctrlKey || ev.metaKey || ev.shiftKey)) {
        // eslint-disable-next-line no-debugger
        debugger;
    }
}

export function installF8Break() {
    window.removeEventListener("keydown", handleF8);
    window.addEventListener("keydown", handleF8);
}

export function uninstallF8Break() {
    window.removeEventListener("keydown", handleF8);
}

export function namedContext<T>(defaultValue: T, name: string) {
    const context = createContext<T>(defaultValue);

    context.displayName = name;
    return context;
}

export function dbg<T>(val: T): T {
    const { stack } = new Error();

    console.log("dbg", val, { stack });
    return val;
}
