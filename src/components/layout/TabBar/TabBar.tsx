import { Clickable } from "@/components/Clickable";
import { AnimateHeight } from "@/components/effects/AnimateHeight";
import { Box } from "@/components/layout/Box";
import { VerticalLine } from "@/components/Lines/VerticalLine";
import { Text } from "@/components/Text";
import { useForceUpdater } from "@/hooks/forceUpdater";
import { useImperativeSprings } from "@/hooks/imperativeSprings";
import { useRect } from "@/hooks/rect";
import { useResizeObserverFromRef } from "@/hooks/resizeObserver";
import { joinWithKey } from "@/utils/array";
import cn from "@/utils/cn";
import { measureRect } from "@/utils/dom/rect";
import { assert } from "@/utils/error";
import { updateRef } from "@/utils/ref";
import { animated } from "@react-spring/web";

import { TabBarPosition } from "./enum";
import styles from "./styles.module.scss";

import { type ReactNode, type RefCallback, useCallback, useEffect, useRef, useState } from "react";

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
    tabsPosition?: TabBarPosition;
}

const positionClasses: Record<TabBarPosition, string> = {
    [TabBarPosition.CENTER]: styles.tabsCenter,
    [TabBarPosition.LEFT]: styles.tabsLeft,
    [TabBarPosition.RIGHT]: styles.tabsRight,
};


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
    setActiveTabRef: RefCallback<HTMLElement>;
    setActiveTab: (tabId: string) => void;
    onTabChange?: (tab: Tab) => void;
    isManaged: boolean;
}

function TabButton({
    tab: tabProp,
    activeTabId,
    className,
    setActiveTabRef,
    setActiveTab,
    onTabChange,
    isManaged,
}: TabButtonProps) {
    const isActive = tabProp.id === activeTabId;
    const ref = useRef<HTMLDivElement>(null);


    const setRef = useCallback((node: HTMLDivElement | null) => {
        updateRef(ref, node);
        if (isActive && node) {
            updateRef(setActiveTabRef, node);
        }
    }, [isActive, setActiveTabRef]);

    return (
        <Clickable
            className={cn(className)}
            onClick={() => {
                const isNew = tabProp.id !== activeTabId;

                if (!isManaged) {
                    setActiveTab(tabProp.id);
                }
                if (isNew) {
                    if (!isManaged) {
                        setActiveTabRef(ref.current);
                    }
                    onTabChange?.(tabProp);
                }
            }}
            ref={setRef}
        >
            {tabProp.renderTab({
                isSelected: isActive,
                selectedTab: activeTabId,
            })}
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
    tabsPosition = TabBarPosition.CENTER,
}: TabBarProps) {
    assert(!(selectedTab && initialSelectedTab), "You can only provide one of selectedTab or initialSelectedTab");

    const [tab, setTab] = useState(selectedTab ?? initialSelectedTab ?? tabs[0]?.id ?? "");
    const [activeTab, setActiveTab] = useState<HTMLElement | null>(null);
    const [layoutMarkerDep, forceLayoutMarker] = useForceUpdater();
    const activeRect = useRect(activeTab);
    const isManaged = selectedTab !== undefined;
    const tabsContainerRef = useRef<HTMLDivElement>(null);

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

    useResizeObserverFromRef(tabsContainerRef, forceLayoutMarker);

    useEffect(() => {
        if (activeRect) {
            const size = activeRect;

            if (x.get() === 0) {
                // FIXME: horrible hack for a bug that i dont even know the cause of
                const size = measureRect(activeTab!);

                width.set(size.width);
                y.set(size.y + size.height);
                x.set(size.x);
            } else {
                width.start(size.width);
                y.start(size.y + size.height);
                x.start(size.x);
            }
        }
    }, [activeRect, activeTab, layoutMarkerDep, width, x, y]);

    return (
        <div className={cn(styles.tabBar, className)}>
            <div
                ref={tabsContainerRef}
                className={cn(styles.tabs, positionClasses[tabsPosition], tabsClassName)}
            >
                {joinWithKey(tabs.map((t) => (
                    <TabButton
                        key={t.id}
                        tab={t}
                        activeTabId={tab}
                        setActiveTab={setTab}
                        onTabChange={onTabChange}
                        isManaged={isManaged}
                        setActiveTabRef={setActiveTab}
                    />
                )), (i) => (noSeparators ? null : <VerticalLine key={`vl-${i}`} />))}
                <animated.div
                    className={styles.marker}
                    style={{
                        x,
                        width,
                        y,
                    }}
                />
            </div>
            <Box
                className={cn(styles.content, contentClassName)}
            >
                <AnimateHeight>
                    {selectedTabObj.render()}
                </AnimateHeight>
            </Box>
        </div>
    );
}
