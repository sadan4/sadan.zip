import { Box } from "@/components/Box";
import { Clickable } from "@/components/Clickable";
import { Text } from "@/components/Text";
import { VerticalLine } from "@/components/VerticalLine";
import { joinWithKey } from "@/utils/array";
import cn from "@/utils/cn";
import { animated, useSpring } from "@react-spring/web";

import invariant from "invariant";
import { type ReactNode, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

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
    noSeparators?: boolean;
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

interface TabButtonProps {
    tab: Tab;
    activeTabId: string;
    className?: string;
    setActiveTabRect: (r?: DOMRect) => void;
    setActiveTab: (tabId: string) => void;
    onTabChange?: (tab: Tab) => void;
}

function TabButton({
    tab: tabProp,
    activeTabId,
    className,
    setActiveTabRect,
    setActiveTab,
    onTabChange,
}: TabButtonProps) {
    const isActive = tabProp.id === activeTabId;
    const ref = useRef<HTMLDivElement>(null);


    const setRef = useCallback((node: HTMLDivElement | null) => {
        ref.current = node;
        if (isActive && node) {
            setActiveTabRect(node.getBoundingClientRect());
        }
    }, [isActive, setActiveTabRect]);

    return (
        <Clickable
            className={cn(className)}
            onClick={() => {
                const isNew = tabProp.id !== activeTabId;

                setActiveTab(tabProp.id);
                if (isNew) {
                    try {
                        onTabChange?.(tabProp);
                    } finally {
                        setActiveTabRect(ref.current?.getBoundingClientRect());
                    }
                }
            }}
            ref={setRef}
        >
            <tabProp.renderTab
                isSelected={isActive}
                selectedTab={activeTabId}
            />
        </Clickable>
    );
}

export function TabBar({
    tabs,
    tabsClassName,
    contentClassName,
    className,
    selectedTab,
    initialSelectedTab,
    onTabChange,
    noSeparators = false,
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


    const [{ width, x, y }, focusBarApi] = useSpring(() => ({
        x: 0,
        width: 0,
        y: 0,
        config: {
        },
    }));

    useLayoutEffect(() => {
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


    return (
        <div className={cn("flex w-full flex-col", className)}>
            <div
                className={cn("flex justify-center gap-2 mb-3", tabsClassName)}
            >
                {joinWithKey(tabs.map((t) => (
                    <TabButton
                        key={t.id}
                        tab={t}
                        activeTabId={tab}
                        setActiveTabRect={setActiveTabRect}
                        setActiveTab={setTab}
                        onTabChange={onTabChange}
                    />
                )), (i) => (noSeparators ? null : <VerticalLine key={`vl-${i}`} />))}
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
