import { useControlledState } from "@/hooks/controlledState";
import cn from "@/utils/cn";
import { NOOP } from "@/utils/constants";
import { namedContext } from "@/utils/devtools";
import { Clickable } from "@components/Clickable";
import { AnimateHeight } from "@effects/AnimateHeight";
import { useForceUpdater } from "@hooks/forceUpdater";
import { animated, useSpring } from "@react-spring/web";

import styles from "./styles.module.scss";
import { AccordionAnimation, ArrowPosition, ClickableArea } from "./utils";

import { type PropsWithChildren, type ReactNode, type Ref, useContext, useEffect, useImperativeHandle, useState } from "react";

export interface AccordionItem {
    id: string;
    contents: ReactNode;
}

export interface AccordionProps extends PropsWithChildren {
    item: AccordionItem;
    className?: string;
    arrowClassName?: string;
    initialOpen?: boolean;
    open?: boolean;
    onToggle?(open: boolean): void;
    clicableArea?: ClickableArea;
    arrowPosition?: ArrowPosition;
    animation?: AccordionAnimation;
}

interface AccordionContext {
    toggleActiveItem(id: string): void;
    getActiveItem(): string | undefined;
    isActive(id: string): boolean | undefined;
    closeAllTrigger: number;
}

const AccordionContext = namedContext<AccordionContext | null>(null, "AccordionContext");

const rotationMap: Record<number, Record<ArrowPosition, number>> = Object.freeze({
    0: {
        [ArrowPosition.RIGHT]: 0,
        [ArrowPosition.LEFT]: -90,
    },
    1: {
        [ArrowPosition.RIGHT]: 180,
        [ArrowPosition.LEFT]: 0,
    },
} satisfies Record<number, Record<ArrowPosition, number>>);

export function Accordion({
    item: { id, contents },
    children,
    className,
    arrowClassName,
    initialOpen = false,
    open: _open,
    arrowPosition = ArrowPosition.RIGHT,
    clicableArea = ClickableArea.ALL,
    onToggle = NOOP,
    animation = AccordionAnimation.ALL,
}: AccordionProps) {
    const [active, setActive] = useControlledState({
        initialValue: initialOpen,
        managedValue: _open,
        handleChange: onToggle,
        debugName: "Accordion.active",
    });

    const groupCtx = useContext(AccordionContext);

    const { rotation } = useSpring({
        rotation: rotationMap[+active][arrowPosition],
        immediate: !(animation & AccordionAnimation.ARROW),
        config: {
            mass: 0.5,
            friction: 50,
        },
    });

    useEffect(() => {
        const num = groupCtx?.closeAllTrigger;

        if (num !== undefined && num !== 0) {
            setActive(false);
        }
    }, [groupCtx?.closeAllTrigger, setActive]);

    useEffect(() => {
        if (!groupCtx) {
            return;
        }

        const isActive = groupCtx.isActive(id);

        if (isActive != null) {
            setActive(isActive);
        }
    }, [groupCtx, id, setActive]);

    function handleClick(area: ClickableArea) {
        if (area & clicableArea) {
            if (groupCtx) {
                groupCtx.toggleActiveItem(id);
            }
            setActive((prev) => !prev);
        }
    }

    const content = (
        <div>
            {active && contents}
        </div>
    );

    return (
        <div className={cn(className)}>
            <Clickable
                className={cn(styles.label, {
                    [styles.right]: arrowPosition === ArrowPosition.RIGHT,
                    [styles.left]: arrowPosition === ArrowPosition.LEFT,
                    [styles.clickableRow]: clicableArea & ClickableArea.ROW,
                    [styles.clickableArrow]: clicableArea & ClickableArea.ARROW,
                })}
                onMouseDown={(e) => {
                    if (clicableArea & ClickableArea.ROW && e.detail > 1) {
                        e.preventDefault();
                    }
                }}
                onClick={() => {
                    handleClick(ClickableArea.ROW);
                }}
            >
                <div>
                    {children}
                </div>
                <animated.svg
                    viewBox="-2.4 -2.4 28.8 28.8"
                    className={cn(styles.arrow, arrowClassName)}
                    style={{
                        transform: rotation.to((r) => `rotate(${r}deg)`),
                    }}
                    onClick={(e) => {
                        e.stopPropagation();
                        handleClick(ClickableArea.ARROW);
                    }}
                >
                    <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth="2"
                        d="m6 9 6 6 6-6"
                    />
                </animated.svg>
            </Clickable>
            {
                animation & AccordionAnimation.CONTENT
                    ? <AnimateHeight>{content}</AnimateHeight>
                    : content
            }
        </div>
    );
}

export interface AccordionGroupProps extends PropsWithChildren {
    activeItemId?: string;
    onItemToggle?: (id: string) => void;
    onlyOneOpen?: boolean;
    ref?: Ref<AccordionGroupHandle>;
}

export interface AccordionGroupHandle {
    closeAll: () => void;
}

export function AccordionGroup({ children, onlyOneOpen = true, ref }: AccordionGroupProps) {
    const [activeItemId, setActiveItemId] = useState<string | undefined>(undefined);
    const [dep, closeItems] = useForceUpdater();

    useImperativeHandle(ref, () => {
        return {
            closeAll() {
                closeItems();
            },
        };
    });

    const api: AccordionContext = {
        toggleActiveItem(id: string): void {
            if (!onlyOneOpen) {
                return;
            }
            setActiveItemId((prev) => (prev === id ? undefined : id));
        },
        getActiveItem(): string | undefined {
            if (!onlyOneOpen) {
                return;
            }
            return activeItemId;
        },
        isActive(id: string): boolean | undefined {
            if (!onlyOneOpen) {
                return;
            }
            if (activeItemId === undefined) {
                return;
            }
            return activeItemId === id;
        },
        closeAllTrigger: dep,
    };

    return (
        <AccordionContext.Provider value={api}>
            {children}
        </AccordionContext.Provider>
    );
}
