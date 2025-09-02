import { Clickable } from "@/components/Clickable";
import { Text } from "@/components/Text";
import { VerticalLine } from "@/components/VerticalLine";
import { useImperativeSprings } from "@/hooks/imperativeSprings";
import { joinWithKey } from "@/utils/array";
import cn from "@/utils/cn";
import useResizeObserver from "@react-hook/resize-observer";
import { animated } from "@react-spring/web";

import { Box } from "../Box";
import { AnimateHeight } from "../effects/AnimateHeight";

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
    isManaged: boolean;
}

function TabButton({
    tab: tabProp,
    activeTabId,
    className,
    setActiveTabRect,
    setActiveTab,
    onTabChange,
    isManaged,
}: TabButtonProps) {
    const isActive = tabProp.id === activeTabId;
    const ref = useRef<HTMLDivElement>(null);


    const setRef = useCallback((node: HTMLDivElement | null) => {
        ref.current = node;
        if (isActive && node) {
            setActiveTabRect(node.getBoundingClientRect());
        }
    }, [isActive, setActiveTabRect]);

    // fixes position of selected tab icon when zooming in/out
    useResizeObserver(ref, () => {
        if (isActive) {
            setActiveTabRect(ref.current?.getBoundingClientRect());
        }
    });


    return (
        <Clickable
            className={cn(className)}
            onClick={() => {
                const isNew = tabProp.id !== activeTabId;

                if (!isManaged) {
                    setActiveTab(tabProp.id);
                }
                if (isNew) {
                    try {
                        onTabChange?.(tabProp);
                    } finally {
                        if (!isManaged) {
                            setActiveTabRect(ref.current?.getBoundingClientRect());
                        }
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
    const lastIndicatorPos = useRef<DOMRect>(null);
    const isManaged = selectedTab !== undefined;


    useEffect(() => {
        if (selectedTab) {
            setTab(selectedTab);
        }
    }, [selectedTab]);

    const selectedTabObj = tabs.find(({ id }) => id === tab) ?? fallbackTab;

    const { x, y, width } = useImperativeSprings({
        x: 0,
        y: 0,
        width: 0,
    });

    useLayoutEffect(() => {
        if (activeTabRect) {
            const size = activeTabRect;

            if (x.get() === 0) {
                width.set(size.width);
                y.set(size.y + size.height);
                x.set(size.x);
                lastIndicatorPos.current = size;
            } else {
                width.start(size.width);
                y.start(size.y + size.height);
                x.start(size.x);
                lastIndicatorPos.current = size;
            }
        }
    }, [activeTabRect, width, x, y]);


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
                        isManaged={isManaged}
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
            <Box
                className={cn("h-full overflow-clip", contentClassName)}
            >
                <AnimateHeight>
                    <selectedTabObj.render />
                </AnimateHeight>
            </Box>
        </div>
    );
}
