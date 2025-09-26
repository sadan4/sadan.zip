import cn from "@/utils/cn";
import { namedContext } from "@/utils/devtools";
import { Clickable } from "@components/Clickable";
import { AnimateHeight } from "@effects/AnimateHeight";
import { useForceUpdater } from "@hooks/forceUpdater";
import { animated, useSpring } from "@react-spring/web";

import { type PropsWithChildren, type ReactNode, type Ref, useContext, useEffect, useImperativeHandle, useState } from "react";

export interface AccordionItem {
    id: string;
    render: () => ReactNode;
}

export interface AccordionProps extends PropsWithChildren {
    item: AccordionItem;
    className?: string;
    initialOpen?: boolean;
}

interface AccordionContext {
    toggleActiveItem(id: string): void;
    getActiveItem(): string | undefined;
    isActive(id: string): boolean | undefined;
    closeAllTrigger: number;
}

const AccordionContext = namedContext<AccordionContext | null>(null, "AccordionContext");

export function Accordion({ item: { id, render: Render }, children, className, initialOpen }: AccordionProps) {
    const [active, setActive] = useState(initialOpen ?? false);
    const groupCtx = useContext(AccordionContext);

    const { rotation } = useSpring({
        rotation: active ? 180 : 0,
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
    }, [groupCtx?.closeAllTrigger]);

    useEffect(() => {
        if (!groupCtx) {
            return;
        }

        const isActive = groupCtx.isActive(id);

        if (isActive != null) {
            setActive(isActive);
        }
    }, [groupCtx, id]);

    return (
        <div className={cn(className)}>
            <Clickable
                className="flex items-center justify-between py-2 pr-2"
                onMouseDown={(e) => {
                    if (e.detail > 1) {
                        e.preventDefault();
                    }
                }}
                onClick={() => {
                    if (groupCtx) {
                        groupCtx.toggleActiveItem(id);
                    }
                    setActive((prev) => !prev);
                }}
            >
                {children}
                <animated.svg
                    viewBox="-2.4 -2.4 28.8 28.8"
                    className="fill-none stroke-fg-500 w-8 h-8"
                    style={{
                        transform: rotation.to((r) => `rotate(${r}deg)`),
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
            <AnimateHeight>
                <div style={{
                    height: active ? "auto" : 0,
                }}
                >
                    <Render />
                </div>
            </AnimateHeight>
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
