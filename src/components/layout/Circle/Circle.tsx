import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";
import { namedContext } from "@/utils/devtools";
import { error } from "@/utils/error";

import { CircleItemContext } from "./context";
import { DefaultPlacementCircleItem } from "./DefaultPlacementCircleItem";
import styles from "./styles.module.scss";

import { type ComponentProps, type PropsWithChildren, type ReactElement, type ReactNode, use, useEffect, useMemo, useState } from "react";

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
    diameter: number;
    numItems?: number;
    offset?: number;
    children: (((props: CircleItemProps) => ReactNode) | ReactNode)[];
}


export interface CircleRootProps extends PropsWithChildren {

}

export interface CircleCenterProps extends PropsWithChildren {
}

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

if (import.meta.hot?.data) {
    (import.meta.hot.data.defaultItems ??= new Set()).add(DefaultPlacementCircleItem);
}

import.meta.hot?.accept(() => {
    if (import.meta.hot) {
        (import.meta.hot.data.defaultItems ??= new Set()).add(DefaultPlacementCircleItem);
    }
});

function isDefaultPlacementCircleItem(component: ReactNode): boolean {
    if (import.meta.env.DEV) {
        if (!import.meta.hot?.data.defaultItems) {
            error("Default items set not found");
        }
        return import.meta.hot.data.defaultItems.has((component as ReactElement)?.type);
    }
    return (component as ReactElement)?.type === DefaultPlacementCircleItem;
}

export function CircleRoot({ children }: CircleRootProps) {
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

export function CircleCenter({ children }: CircleCenterProps) {
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

export function CircleItems({
    diameter,
    children,
    numItems: _numItems,
    offset = 0,
    ...props
}: CircleItemsProps) {
    // FIXME: Weird workaround to make react compiler happy
    const numItems = _numItems ?? children.length;
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
                width: diameter,
                height: diameter,
                top: top + ((height / 2) - (diameter / 2)),
                left: left + ((width / 2) - (diameter / 2)),
            }}
        >
            {Array.from({ length: numItems }, (_, i) => {
                const child = children[i];

                if (!child) {
                    return null;
                }

                const angle = angleStep * (i + offset);
                const x = (diameter / 2) + ((diameter / 2) * Math.cos(angle));
                const y = (diameter / 2) + ((diameter / 2) * Math.sin(angle));
                const lastAngle = angleStep * (i + (i ? offset - 1 : offset - children.length - 1));
                const lastX = (diameter / 2) + ((diameter / 2) * Math.cos(lastAngle));
                const lastY = (diameter / 2) + ((diameter / 2) * Math.sin(lastAngle));
                const nextAngle = angleStep * (i + (i < numItems - 1 ? offset + 1 : offset - children.length + 1));
                const nextX = (diameter / 2) + ((diameter / 2) * Math.cos(nextAngle));
                const nextY = (diameter / 2) + ((diameter / 2) * Math.sin(nextAngle));

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

                const placementProps: CircleItemProps = Object.freeze({
                    x,
                    y,
                    n: i,
                    radius: diameter,
                    angle,
                    nextItem,
                    lastItem,
                });

                if (typeof child !== "function") {
                    return (
                        <CircleItemContext
                            value={placementProps}
                            key={(child as ReactElement)?.key}
                        >
                            {isDefaultPlacementCircleItem(child)
                                ? child
                                : (
                                    <DefaultPlacementCircleItem>
                                        {child}
                                    </DefaultPlacementCircleItem>
                                )}
                        </CircleItemContext>
                    );
                }

                const c = (child ?? (() => null))(placementProps);

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

