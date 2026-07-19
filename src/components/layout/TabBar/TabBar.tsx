import { Clickable } from "@/components/Clickable";
import { AnimateHeight } from "@/components/effects/AnimateHeight";
import { Box } from "@/components/layout/Box";
import { VerticalLine } from "@/components/Lines/VerticalLine";
import { Text } from "@/components/Text";
import { joinWithKey } from "@/utils/array";
import cn from "@/utils/cn";
import { assert } from "@/utils/error";

import { TabBarPosition } from "./enum";
import * as styles from "./styles.module.scss";

import { type ReactNode, useEffect, useState } from "react";

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
    onTabChange?(tab: Tab): void;
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
    setActiveTab(tabId: string): void;
    onTabChange?(tab: Tab): void;
    isManaged: boolean;
}

function TabButton({
    tab: tabProp,
    activeTabId,
    className,
    setActiveTab,
    onTabChange,
    isManaged,
}: TabButtonProps) {
    const isActive = tabProp.id === activeTabId;

    return (
        <Clickable
            className={cn(className, isActive && styles.selectedTab)}
            onClick={() => {
                const isNew = tabProp.id !== activeTabId;

                if (!isManaged) {
                    setActiveTab(tabProp.id);
                }
                if (isNew) {
                    onTabChange?.(tabProp);
                }
            }}
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

    const [tab, setTab] = useState(selectedTab ?? initialSelectedTab ?? (tabs[0] as Tab | undefined)?.id ?? "");
    const isManaged = selectedTab !== undefined;

    useEffect(() => {
        if (selectedTab) {
            // oxlint-disable-next-line react/react-compiler
            setTab(selectedTab);
        }
    }, [selectedTab]);

    const selectedTabObj = tabs.find(({ id }) => id === tab) ?? fallbackTab;

    return (
        <div className={cn(styles.tabBar, className)}>
            <div
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
                    />
                )), (i) => (noSeparators ? null : <VerticalLine key={`vl-${i}`} />))}
                <div className={styles.marker} />
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
