import { useEffect } from "react";

const cssMap = Object.create(null) as Record<string, [element: HTMLLinkElement, refCount: number]>;

export function useCssFile(file: string): void {
    useEffect(() => {
        if (file in cssMap) {
            cssMap[file][1]++;
        } else {
            const styleElement = document.createElement("link");

            styleElement.rel = "stylesheet";
            styleElement.href = file;

            document.head.appendChild(styleElement);
            cssMap[file] = [styleElement, 1];
        }
        return () => {
            if (file in cssMap) {
                const [element, refCount] = cssMap[file];

                if (refCount <= 1) {
                    document.head.removeChild(element);
                    delete cssMap[file];
                } else {
                    cssMap[file][1]--;
                }
            }
        };
    }, [file]);
}
