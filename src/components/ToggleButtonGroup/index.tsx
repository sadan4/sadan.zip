import { useControlledState } from "@/hooks/controlledState";
import cn from "@/utils/cn";

import * as styles from "./styles.module.scss";
import { Clickable } from "../Clickable";
import { Tooltip } from "../Tooltip";
import type { TooltipPosition } from "../Tooltip/constants";

import type { ComponentProps, ReactNode } from "react";

export interface ToggleButton<T extends string | number> {
    id: T;
    label: string;
    renderIcon(): ReactNode;
}
export interface ToggleButtonGroupProps<T extends string | number> extends ComponentProps<"div"> {
    items: ToggleButton<T>[];
    selectedItem?: T;
    initialSelectedItem?: T;
    onSelectItem?(item: T): void;
    children?: never;
    tooltipPosition?: TooltipPosition;
    /**
     * class to be added to each item
     */
    itemClassName?: string;
}

export function ToggleButtonGroup<T extends string | number>({
    items,
    selectedItem: controlledSelectedItem,
    initialSelectedItem,
    onSelectItem,
    tooltipPosition,
    className,
    itemClassName,
    ...props
}: ToggleButtonGroupProps<T>) {
    const [selectedItem, setSelectedItem] = useControlledState<T>({
        initialValue: initialSelectedItem ?? items[0].id,
        managedValue: controlledSelectedItem,
        handleChange(newValue) {
            onSelectItem?.(newValue);
        },
        debugName: "ToggleButtonGroup.SelectedItem",
    });

    return (
        <div
            className={cn(styles.wrapper, className)}
            {...props}
        >
            <div className={cn(styles.marker)} />
            <div className={styles.buttons}>
                {items.map(({ id, label, renderIcon }) => {
                    const isSelected = id === selectedItem;

                    return (
                        <div
                            key={id}
                            className={cn(isSelected && styles.selected, itemClassName)}
                        >
                            <Tooltip
                                text={label}
                                position={tooltipPosition}
                            >
                                <Clickable
                                    onClick={() => {
                                        setSelectedItem(id);
                                    }}
                                >
                                    {renderIcon()}
                                </Clickable>
                            </Tooltip>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
