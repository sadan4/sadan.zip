import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";
import { error } from "@/utils/error";
import { polarToCartesian } from "@/utils/math";

import { CircleItemContext } from "./context";
import { DefaultPlacementCircleItem } from "./DefaultPlacementCircleItem";
import * as styles from "./styles.module.scss";

import { type ComponentProps, createContext, type PropsWithChildren, type ReactElement, type ReactNode, use, useEffect, useMemo, useState } from "react";

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
    rect?: DOMRectReadOnly;
}

interface InternalCircleContext {
    rect: DOMRectReadOnly | undefined;
    setRect(rect: DOMRectReadOnly | undefined): void;
}

const InternalCircleContext = createContext<InternalCircleContext | null>(null);

InternalCircleContext.displayName = "InternalCircleContext";

function useCircleContextInternal(): InternalCircleContext {
    const ctx = use(InternalCircleContext);

    if (ctx == null) {
        error("useCircleContextInternal must be used within a Circle.Root");
    }

    return ctx;
}

interface HotData {
    defaultItems?: Set<any>;
}

// if (import.meta.hot?.data) {
//     (import.meta.hot.data.defaultItems ??= new Set()).add(DefaultPlacementCircleItem);
// }

if (import.meta.env.DEV && import.meta.webpackHot) {
    import.meta.webpackHot.data ??= Object.create(null);
}

if (import.meta.webpackHot?.data) {
    ((import.meta.webpackHot.data as HotData).defaultItems ??= new Set()).add(DefaultPlacementCircleItem);
}

// if (import.meta.hot) {
//     import.meta.hot.accept(() => {
//         if (import.meta.hot) {
//             (import.meta.hot.data.defaultItems ??= new Set()).add(DefaultPlacementCircleItem);
//         }
//     });
// }

if (import.meta.webpackHot) {
    import.meta.webpackHot.dispose((data) => {
        ((data as HotData).defaultItems ??= new Set()).add(DefaultPlacementCircleItem);
    });
}

function isDefaultPlacementCircleItem(component: ReactNode): boolean {
    if (import.meta.env.DEV && import.meta.webpackHot) {
        // if (!import.meta.hot?.data.defaultItems) {
        //     error("Default items set not found");
        // }
        // return import.meta.hot.data.defaultItems.has((component as ReactElement)?.type);
        if (!(import.meta.webpackHot.data as HotData | undefined)?.defaultItems) {
            error("Default items set not found");
        }
        return (import.meta.webpackHot!.data as HotData).defaultItems!.has((component as ReactElement)?.type);
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
        <InternalCircleContext value={api}>
            {children}
        </InternalCircleContext>
    );
}

export function CircleCenter({ children, rect: rectProp }: CircleCenterProps) {
    const [el, setEl] = useState<HTMLDivElement | null>(null);
    const rect = useRect(el);
    const circleCtx = useCircleContextInternal();

    useEffect(() => {
        circleCtx.setRect(rectProp ?? rect);
    }, [rectProp, rect, circleCtx]);

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
    const radius = diameter / 2;

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
                top: top + ((height / 2) - radius),
                left: left + ((width / 2) - radius),
            }}
        >
            {Array.from({ length: numItems }, (_, i) => {
                const child = children[i];

                if (!child) {
                    return null;
                }

                const angle = angleStep * (i + offset);
                const lastAngle = angleStep * (i + (i ? offset - 1 : offset - children.length - 1));
                const nextAngle = angleStep * (i + (i < numItems - 1 ? offset + 1 : offset - children.length + 1));
                let [x, y] = polarToCartesian(radius, angle);
                let [lastX, lastY] = polarToCartesian(radius, lastAngle);
                let [nextX, nextY] = polarToCartesian(radius, nextAngle);

                // we need to offset every value by radius because the values
                // are calculated assuming the center of the circle is the origin
                // but the origin is actually at the top left

                x += radius;
                y += radius;
                lastX += radius;
                lastY += radius;
                nextX += radius;
                nextY += radius;

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
                    <CircleItemContext
                        value={placementProps}
                        key={(c as ReactElement)?.key}
                    >
                        {c}
                    </CircleItemContext>
                );
            })}
        </div>
    );
}

