import { error } from "@/utils/error";

import { useRecent } from "./recent";

import { useEffect } from "react";

export interface UseConsoleHelpersOptions {
    /**
     * @default false
     */
    silenceWarnings?: boolean;
    /**
     * @default "setters-only"
     */
    settable?: boolean | "setters-only";
}

export function useConsoleHelpers(helpers: Record<string, any>, { settable = "setters-only", silenceWarnings = false }: UseConsoleHelpersOptions = {}) {
    const helpersRef = useRecent(helpers);
    const settableRef = useRecent(settable);

    useEffect(() => {
        const o = helpersRef.current;
        const keys = Object.keys(o);

        console.log(`${keys.join(", ")} available on globalThis for debugging`);

        const setKeys = keys
            .filter((key) => {
                if (key in globalThis) {
                    if (!silenceWarnings) {
                        console.warn(`key "${key}" already exists on globalThis, ignoring`);
                    }
                    return false;
                }
                return true;
            });

        for (const key of setKeys) {
            Object.defineProperty(globalThis, key, {
                configurable: true,
                enumerable: true,
                get() {
                    return helpersRef.current[key];
                },
                set(value: any) {
                    switch (settableRef.current) {
                        case "setters-only": {
                            const noSetter = Object.getOwnPropertyDescriptor(o, key)?.set == null;

                            if (noSetter) {
                                error(`cannot set ${key}`);
                            }
                            // fallthrough
                        }
                        case true:
                            helpersRef.current[key] = value;
                            break;
                        case false:
                            error(`cannot set ${key}`);
                            break;
                        default:
                            error("unhandled case");
                    }
                },
            });
        }
        return () => {
            for (const key of setKeys) {
                delete (globalThis as any)[key];
            }
        };
    }, [
        helpersRef,
        settableRef,
        silenceWarnings,
        // eslint-disable-next-line react-hooks/exhaustive-deps
        ...Object.keys(helpers),
    ]);
}
