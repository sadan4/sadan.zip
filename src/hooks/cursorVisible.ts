import { useEffect, useState } from "react";

const enterHandlers = new Set<() => void>();
const leaveHandlers = new Set<() => void>();

window.addEventListener("mouseover", () => {
    enterHandlers.forEach((handler) => handler());
});

window.addEventListener("mousemove", () => {
    enterHandlers.forEach((handler) => handler());
});

window.addEventListener("mouseout", () => {
    leaveHandlers.forEach((handler) => handler());
});

export function useCursorVisible(defaultValue?: boolean) {
    const [visible, setVisible] = useState(defaultValue !== undefined ? defaultValue : document.body.matches(":hover"));

    // mousemove instead of mouseover because react 
    // devtools blocks mouseover events while inspecting
    useEffect(() => {
        function enter() {
            setVisible(true);
        }
        function leave() {
            setVisible(false);
        }
        enterHandlers.add(enter);
        leaveHandlers.add(leave);
        return () => {
            enterHandlers.delete(enter);
            leaveHandlers.delete(leave);
        };
    });

    return visible;
}
