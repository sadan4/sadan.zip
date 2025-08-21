import { joinWithKey } from "@/utils/array";
import cn from "@/utils/cn";
import { animated, useSpring } from "@react-spring/web";

import { Box } from "./Box";
import { Clickable } from "./Clickable";
import { Text } from "./Text";
import { VerticalLine } from "./VerticalLine";

import invariant from "invariant";
import { type ReactNode, useEffect, useRef, useState } from "react";

export interface TabRowItemProps {
    isSelected: boolean;
    selectedTab: string;
}

export interface Tab {
    readonly id: string;
    renderTab(props: TabRowItemProps): ReactNode;
    render(): ReactNode;
}
export interface TabBarProps {
    tabs: Tab[];
    className?: string;
    tabsClassName?: string;
    contentClassName?: string;
    selectedTab?: string;
    initialSelectedTab?: string;
    onTabChange?: (tab: Tab) => void;
}

const fallbackTab: Tab = {
    id: "FALLBACK_TAB",
    render() {
        return (
            <Text
                size="xl"
                color="error"
                weight="bold"
            >
                ERROR
            </Text>
        );
    },
    renderTab() {
        return (
            <Text
                size="lg"
                color="error"
                weight="bold"
            >
                ERROR
            </Text>
        );
    },
};

export function TabBar({
    tabs,
    tabsClassName,
    contentClassName,
    className,
    selectedTab,
    initialSelectedTab,
    onTabChange,
}: TabBarProps) {
    invariant(!(selectedTab && initialSelectedTab), "You can only provide one of selectedTab or initialSelectedTab");

    const [tab, setTab] = useState(selectedTab ?? initialSelectedTab ?? (tabs[0]?.id || ""));
    const [activeTabRect, setActiveTabRect] = useState<DOMRect | undefined>();

    useEffect(() => {
        if (selectedTab) {
            setTab(selectedTab);
        }
    }, [selectedTab]);

    const selectedTabObj = tabs.find(({ id }) => id === tab) ?? fallbackTab;

    interface TabButtonProps {
        tab: Tab;
    }


    const [{ width, x, y }, focusBarApi] = useSpring(() => ({
        x: 0,
        width: 0,
        y: 0,
        config: {
        },
    }));

    useEffect(() => {
        if (activeTabRect) {
            const size = activeTabRect;

            if (x.get() === 0) {
                focusBarApi.set({
                    width: size.width,
                    x: size.x,
                    y: size.y + size.height,
                });
            } else {
                focusBarApi.start({
                    width: size.width,
                    x: size.x,
                    y: size.y + size.height,
                });
            }
        }
    }, [activeTabRect, focusBarApi, x]);

    function TabButton({ tab: tabProp }: TabButtonProps) {
        const isActive = tabProp.id === tab;
        const ref = useRef<HTMLDivElement>(null);

        return (
            <Clickable
                className={cn(tabsClassName)}
                onClick={(e) => {
                    const isNew = tabProp.id !== tab;

                    setTab(tabProp.id);
                    if (isNew) {
                        try {
                            onTabChange?.(tabProp);
                        } finally {
                            setActiveTabRect(ref.current?.getBoundingClientRect());
                        }
                    }
                }}
                ref={(e) => {
                    ref.current = e;
                    if (isActive && e && !activeTabRect) {
                        setActiveTabRect(e.getBoundingClientRect());
                    }
                }}
            >
                <tabProp.renderTab
                    isSelected={isActive}
                    selectedTab={tab}
                />
            </Clickable>
        );
    }

    return (
        <div className={cn("flex w-full flex-col", className)}>
            <div
                className={cn("flex justify-center gap-2 mb-3", tabsClassName)}
            >
                {joinWithKey(tabs.map((t) => (
                    <TabButton
                        key={t.id}
                        tab={t}
                    />
                )), (i) => <VerticalLine key={`vl-${i}`} />)}
                <animated.div
                    className="absolute top-0 left-0 pointer-events-none border-b-2 border-bg-fg-500"
                    style={{
                        x,
                        width,
                        y,
                    }}
                />
            </div>
            <Box className={cn("h-full", contentClassName)}>
                <selectedTabObj.render />
            </Box>
        </div>
    );
}
