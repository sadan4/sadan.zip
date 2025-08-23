import { useForceUpdater } from "@/hooks/forceUpdater";
import cn from "@/utils/cn";
import { animated, useSpringValue } from "@react-spring/web";

import { Box } from "./Box";
import { Clickable } from "./Clickable";

import { createContext, type PropsWithChildren, type ReactNode, type Ref, useContext, useEffect, useImperativeHandle, useState } from "react";

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
    effectAndCloseWhenChangeAndNotZero: number;
}

const AccordionContext = createContext<AccordionContext | null>(null);

export function Accordion({ item: { id, render: Render }, children, className, initialOpen }: AccordionProps) {
    const [active, setActive] = useState(initialOpen ?? false);
    const accordionApi = useContext(AccordionContext);

    const rotation = useSpringValue(active ? 180 : 0, {
        config: {
            mass: 0.5,
            friction: 50,
        },
    });

    useEffect(() => {
        const num = accordionApi?.effectAndCloseWhenChangeAndNotZero;

        if (num !== undefined && num !== 0) {
            setActive(false);
        }
    }, [accordionApi?.effectAndCloseWhenChangeAndNotZero]);

    useEffect(() => {
        rotation.start(active ? 180 : 0);
    }, [active, rotation]);

    useEffect(() => {
        if (!accordionApi) {
            return;
        }

        const isActive = accordionApi.isActive(id);

        if (isActive != null) {
            setActive(isActive);
        }
    }, [accordionApi, id]);

    return (
        <Box className={cn(className)}>
            <Clickable
                // -mb-2 to offset box looking weird
                className="flex items-center justify-between py-2 pr-2 -mb-2"
                onMouseDown={(e) => {
                    if (e.detail > 1) {
                        e.preventDefault();
                    }
                }}
                onClick={() => {
                    if (accordionApi) {
                        accordionApi.toggleActiveItem(id);
                    }
                    setActive((prev) => !prev);
                }}
            >
                {children}
                <animated.svg
                    viewBox="-2.4 -2.4 28.8 28.8"
                    className="fill-none stroke-bg-fg w-8 h-8"
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
            <div>
                {active && <Render />}
            </div>
        </Box>
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
        effectAndCloseWhenChangeAndNotZero: dep,
    };

    return (
        <AccordionContext.Provider value={api}>
            {children}
        </AccordionContext.Provider>
    );
}
