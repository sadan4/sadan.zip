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
