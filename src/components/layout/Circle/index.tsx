import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";
import { namedContext, namespacedComponent } from "@/utils/devtools";
import { error } from "@/utils/error";

import { CircleItemContext } from "./context";
import styles from "./styles.module.scss";

import { type ComponentProps, type PropsWithChildren, type ReactElement, type ReactNode, use, useContext, useEffect, useMemo, useState } from "react";

export interface CircleItemProps {
    x: number;
    y: number;
    n: number;
    radius: number;
    angle: number;
    lastItem: Pick<CircleItemProps, "x" | "y" | "angle">;
    nextItem: Pick<CircleItemProps, "x" | "y" | "angle">;
}

export interface CircleItemsProps extends Omit<ComponentProps<"div">, "children"> {
    radius: number;
    numItems?: number;
    offset?: number;
    children: (((props: CircleItemProps) => ReactNode) | ReactNode)[];
}


export interface CircleRootProps extends PropsWithChildren {

}

export interface CircleCenterProps extends PropsWithChildren {
}

export namespace Circle {
    interface CircleContextInternal {
        rect: DOMRectReadOnly | undefined;
        setRect(rect: DOMRectReadOnly | undefined): void;
    }

    const CircleContextInternal = namedContext<CircleContextInternal | null>(null, "CircleContextInternal");

    function useCircleContextInternal(): CircleContextInternal {
        const ctx = use(CircleContextInternal);

        if (ctx == null) {
            error("useCircleContextInternal must be used within a Circle.Root");
        }

        return ctx;
    }

    export function Root({ children }: CircleRootProps) {
        const [rect, setRect] = useState<DOMRectReadOnly>();

        const api = useMemo(() => ({
            rect,
            setRect,
        }), [rect]);

        return (
            <CircleContextInternal value={api}>
                {children}
            </CircleContextInternal>
        );
    }

    export function Center({ children }: CircleCenterProps) {
        const [el, setEl] = useState<HTMLDivElement | null>(null);
        const rect = useRect(el);
        const circleCtx = useCircleContextInternal();

        useEffect(() => {
            circleCtx.setRect(rect);
        }, [rect, circleCtx]);

        return (
            <div
                className="contents"
                ref={setEl}
            >
                {children}
            </div>
        );
    }

    export function DefaultPlacementCircleItem({ children }: PropsWithChildren) {
        const { x: left, y: top } = useContext(CircleItemContext);

        return (
            <div
                className={styles.default}
                style={{
                    top,
                    left,
                }}
            >
                {children}
            </div>
        );
    }

    export function Items({ radius, children, numItems = children.length, offset = 0, ...props }: CircleItemsProps) {
        const angleStep = (2 * Math.PI) / numItems;
        const { rect: { top = 0, left = 0, width = 0, height = 0 } = {} } = useCircleContextInternal();

        if (children.length === 0)
            return null;

        return (
            <div
                {...props}
                className={cn(props.className, styles.items)}
                style={{
                    ...props.style,
                    width: radius,
                    height: radius,
                    top: top + ((height / 2) - (radius / 2)),
                    left: left + ((width / 2) - (radius / 2)),
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
                            <CircleItemContext
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
                            </CircleItemContext>
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
}

namespacedComponent(Circle, "Circle");

