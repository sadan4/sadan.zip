import toCSS from "@/utils/toCSS";

import type { PropsWithChildren, ReactNode } from "react";

export interface CircleItemProps {
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
    children: (((props: CircleItemProps) => ReactNode) | ReactNode)[];
}

export function DefaultPlacementCircleItem({ x, y, children }: PropsWithChildren<CircleItemProps>) {
    return (
        <div
            style={{
                top: toCSS.px(y),
                left: toCSS.px(x),
            }}
        >
            {children}
        </div>
    );
}

export default function Circle({ radius, children, numItems = children.length, offset = 0 }: CircleProps) {
    const angleStep = (2 * Math.PI) / numItems;

    if (children.length === 0)
        return null;

    return (
        <div
            className="-translate-1/2 *:-translate-1/2 *:absolute pointer-events-none *:pointer-events-auto"
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

                if (typeof children[i] !== "function") {
                    return (
                        <DefaultPlacementCircleItem
                            x={x}
                            y={y}
                            n={i}
                            radius={radius}
                            angle={angle}
                        >
                            {children[i]}
                        </DefaultPlacementCircleItem>
                    );
                }
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
