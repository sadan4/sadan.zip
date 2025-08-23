import type { Ref } from "react";

export function updateRef<T>(ref: Ref<T> | undefined, value: T) {
    if (ref == null) {
        return;
    } else if (typeof ref === "function") {
        try {
            ref(value);
        } catch (e) {
            console.error("Error occurred while updating ref:", e);
        }
    } else {
        ref.current = value;
    }
}
