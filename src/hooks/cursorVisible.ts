import { useEventHandler } from "./eventListener";

import { useState } from "react";

export function useCursorVisible() {
    const [visible, setVisible] = useState(document.body.matches(":hover"));

    useEventHandler("mouseover", () => {
        setVisible(true);
    });
    useEventHandler("mouseout", () => {
        setVisible(false);
    });
    return visible;
}
