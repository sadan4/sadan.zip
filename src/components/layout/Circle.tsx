import toCSS from "@/utils/toCSS";

import type { ReactNode } from "react";

interface CircleItemProps {
    x: number;
    y: number;
    n: number;
    radius: number;
    angle: number;
}

export interface CircleProps {
    radius: number;
    numItems?: number;
    offset?: number;
    children: ((props: CircleItemProps) => ReactNode)[];
}

export default function Circle({ radius, children, numItems = children.length, offset = 0 }: CircleProps) {
    const angleStep = (2 * Math.PI) / numItems;

    if (children.length === 0)
        return null;

    return (
        <div
            className="-translate-1/2 *:-translate-1/2 *:absolute pointer-events-none"
            style={{
                position: "absolute",
                width: toCSS.px(radius),
                height: toCSS.px(radius),
            }}
        >
            {Array.from({ length: numItems }, (_, i) => {
                const angle = angleStep * (i + offset);
                const x = (radius / 2) + ((radius / 2) * Math.cos(angle));
                const y = (radius / 2) + ((radius / 2) * Math.sin(angle));

                return (children[i] ?? (() => null))({
                    x,
                    y,
                    n: i,
                    radius,
                    angle,
                });
            })}
        </div>
    );
}
