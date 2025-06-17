import { disposableEventHandler } from "@/utils/events";

import { useEffect, useState } from "react";

export function useCursorVisible() {
    const [visible, setVisible] = useState(document.body.matches(":hover"));

    // mousemove instead of mouseover because react 
    // devtools blocks mouseover events while inspecting
    useEffect(() => {
        if (visible) {
            return disposableEventHandler("mouseout", () => {
                setVisible(false);
            });
        }
        return disposableEventHandler("mousemove", () => {
            setVisible(true);
        });
    }, [visible]);

    return visible;
}
