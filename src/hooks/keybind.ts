import { useRecent } from "./recent";


export const enum KeybindModifiers {
    NONE = 0,
    // cmd on mac
    CTRL = 1 << 0,
    ALT = 1 << 1,
    SHIFT = 1 << 2,
    // ctrl on mac
    META = 1 << 3,
}

// https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values
// https://www.toptal.com/developers/keycode
export namespace KeybindKeys {
    export const ESCAPE = "Escape";

    export const A = "a";
    export const B = "b";
    export const C = "c";
    export const D = "d";
    export const E = "e";
    export const F = "f";
    export const G = "g";
    export const H = "h";
    export const I = "i";
    export const J = "j";
    export const K = "k";
    export const L = "l";
    export const M = "m";
    export const N = "n";
    export const O = "o";
    export const P = "p";
    export const Q = "q";
    export const R = "r";
    export const S = "s";
    export const T = "t";
    export const U = "u";
    export const V = "v";
    export const W = "w";
    export const X = "x";
    export const Y = "y";
    export const Z = "z";
}

export interface Keybind {
    /**
     * @see {@link KeybindModifiers}
     */
    modifiers?: number;
    /**
     * The key to bind to.
     * 
     * @see {@link KeybindKeys}
     */
    key: typeof KeybindKeys[keyof typeof KeybindKeys];
    handle(ev: KeyboardEvent): void;
    /**
     * Whether the keybind should be handled on keydown or keyup.
     */
    timing: "down" | "up";
}

export function makeModifierMask(ev: KeyboardEvent): number {
    let mask = KeybindModifiers.NONE;

    if (ev.ctrlKey)
        mask |= KeybindModifiers.CTRL;
    if (ev.altKey)
        mask |= KeybindModifiers.ALT;
    if (ev.shiftKey)
        mask |= KeybindModifiers.SHIFT;
    if (ev.metaKey)
        mask |= KeybindModifiers.META;

    return mask;
}

export function matchesEvent(ev: KeyboardEvent, keybind: Keybind): boolean {
    if (ev.key !== keybind.key)
        return false;

    return makeModifierMask(ev) === (keybind.modifiers ?? KeybindModifiers.NONE);
}

export function useKeybinds(keybinds: Keybind[]): (ref: HTMLElement | null) => () => void {
    const keybindsRef = useRecent(keybinds);

    function callValidBinds(mode: "down" | "up", ev: KeyboardEvent) {
        return keybindsRef.current
            .filter((kb) => {
                return kb.timing === mode && matchesEvent(ev, kb);
            })
            .forEach((kb) => {
                kb.handle(ev);
            });
    }

    return (ref: HTMLElement | null) => {
        if (ref == null) {
            return () => {};
        }

        const controller = new AbortController();

        ref.addEventListener("keydown", callValidBinds.bind(null, "down"), {
            passive: true,
            signal: controller.signal,
        });
        ref.addEventListener("keyup", callValidBinds.bind(null, "up"), {
            passive: true,
            signal: controller.signal,
        });

        return () => {
            controller.abort();
        };
    };
}
