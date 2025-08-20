import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";

import type { ComponentProps, JSX, PropsWithChildren, ReactNode } from "react";

export interface CircleItemProps {
    x: number;
    y: number;
    n: number;
    radius: number;
    angle: number;
    lastItem: Pick<CircleItemProps, "x" | "y" | "angle">;
    nextItem: Pick<CircleItemProps, "x" | "y" | "angle">;
}

export interface CircleProps extends Omit<ComponentProps<"div">, "children"> {
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

export default function Circle({ radius, children, numItems = children.length, offset = 0, ...props }: CircleProps) {
    const angleStep = (2 * Math.PI) / numItems;

    if (children.length === 0)
        return null;

    return (
        <div
            {...props}
            className={cn(props.className, "-translate-1/2 *:-translate-1/2 *:absolute pointer-events-none *:pointer-events-auto")}
            style={{
                ...props.style,
                position: "absolute",
                width: toCSS.px(radius),
                height: toCSS.px(radius),
            }}
        >
            {Array.from({ length: numItems }, (_, i) => {
                const angle = angleStep * (i + offset);
                const x = (radius / 2) + ((radius / 2) * Math.cos(angle));
                const y = (radius / 2) + ((radius / 2) * Math.sin(angle));
                const lastAngle = angleStep * (i + (i ? offset - 1 : offset - children.length - 1));
                const lastX = (radius / 2) + ((radius / 2) * Math.cos(lastAngle));
                const lastY = (radius / 2) + ((radius / 2) * Math.sin(lastAngle));
                const nextAngle = angleStep * (i + (i < numItems - 1 ? offset + 1 : offset - children.length + 1));
                const nextX = (radius / 2) + ((radius / 2) * Math.cos(nextAngle));
                const nextY = (radius / 2) + ((radius / 2) * Math.sin(nextAngle));

                const nextItem = {
                    x: nextX,
                    y: nextY,
                    angle: nextAngle,
                };

                const lastItem = {
                    x: lastX,
                    y: lastY,
                    angle: lastAngle,
                };

                if (children[i] == null) {
                    return null;
                }
                if (typeof children[i] !== "function") {
                    return (
                        <DefaultPlacementCircleItem
                            x={x}
                            y={y}
                            n={i}
                            radius={radius}
                            angle={angle}
                            nextItem={nextItem}
                            lastItem={lastItem}
                            key={(children[i] as JSX.Element)?.key}
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
                    nextItem,
                    lastItem,
                });
            })}
        </div>
    );
}
