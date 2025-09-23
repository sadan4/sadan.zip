import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";

import { CircleItemContext } from "./context";

import { type ComponentProps, type PropsWithChildren, type ReactElement, type ReactNode, useContext } from "react";

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

export function DefaultPlacementCircleItem({ children }: PropsWithChildren) {
    const { x: left, y: top } = useContext(CircleItemContext);

    return (
        <div
            style={{
                top,
                left,
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

                const placementProps: CircleItemProps = Object.freeze({
                    x,
                    y,
                    n: i,
                    radius,
                    angle,
                    nextItem,
                    lastItem,
                });

                if (typeof children[i] !== "function") {
                    return (
                        <CircleItemContext.Provider
                            value={placementProps}
                            key={(children[i] as ReactElement)?.key}
                        >
                            {(children[i] as ReactElement)?.type === DefaultPlacementCircleItem
                                ? children[i]
                                : (
                                    <DefaultPlacementCircleItem>
                                        {children[i]}
                                    </DefaultPlacementCircleItem>
                                )}
                        </CircleItemContext.Provider>
                    );
                }

                const c = (children[i] ?? (() => null))(placementProps);

                return (
                    <CircleItemContext.Provider
                        value={placementProps}
                        key={(c as ReactElement)?.key}
                    >
                        {c}
                    </CircleItemContext.Provider>
                );
            })}
        </div>
    );
}
