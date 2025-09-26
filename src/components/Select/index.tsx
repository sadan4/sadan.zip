import { useControlledState } from "@/hooks/controlledState";
import { border, z } from "@/styles";
import cn from "@/utils/cn";
import { prop } from "@/utils/functional";
import { Input } from "@components/Input";
import { animated, useSpringValue } from "@react-spring/web";

import styles from "./styles.module.scss";
import { Clickable } from "../Clickable";
import CheckCircle from "../icons/CheckCircle";
import { ScrollArea } from "../layout/ScrollArea";
import { ScrollAreaContext } from "../layout/ScrollArea/context";
import { Text } from "../Text";

import invariant from "invariant";
import { type Key, type PropsWithChildren, type ReactNode, useContext, useEffect, useRef, useState } from "react";
import scrollIntoView from "scroll-into-view-if-needed";

export interface SelectOption<T> {
    disabled?: boolean;
    value: T;
    label: ReactNode;
    /**
     * The value to search by when using the keyboard to select an item
     */
    typedValue: string | null;
    key?: Key;
    default?: boolean;
}

interface SelectItemProps<T> {
    item: SelectOption<T>;
    isSelected: boolean;
    onChange: (item: NoInfer<T>) => void;
}

function SelectItem<T>({ item: { label, value, disabled }, isSelected, onChange }: SelectItemProps<T>) {
    const ref = useRef<HTMLDivElement>(null);
    const ctx = useContext(ScrollAreaContext).ref;

    useEffect(() => {
        if (isSelected && ref.current) {
            scrollIntoView(ref.current, {
                boundary: ctx.current,
                scrollMode: "if-needed",
                behavior: "instant",
            });
        }
    }, [ctx, isSelected]);

    return (
        <Clickable
            ref={ref}
            className={cn("flex items-center p-2", disabled ? "brightness-50" : "hover:bg-bg-200")}
            onClick={() => onChange(value)}
        >
            <div className="flex-1/1">{label}</div>
            {isSelected && <CheckCircle className="fill-info-500"/>}
        </Clickable>
    );
}

interface SelectMenuProps<T> {
    items: SelectOption<T>[];
    selectedItem: NoInfer<T>;
    scrollAreaClassName?: string;
    onChange: (item: NoInfer<T>) => void;
}

function SelectMenu<T>({ items, onChange, selectedItem, scrollAreaClassName }: SelectMenuProps<T>) {
    return (
        <ScrollArea className={cn("bg-bg-300 sb-track-bg-300 rounded-md border-fg-700 border-3 flex flex-col", scrollAreaClassName)}>
            {items.map((item) => {
                return (
                    <SelectItem
                        key={item.key}
                        onChange={onChange}
                        isSelected={selectedItem === item.value}
                        item={item}
                    />
                );
            })}
        </ScrollArea>
    );
}


function getDefaultItem<T>(items: SelectOption<T>[]): SelectOption<T> {
    return items.find(prop("default")) ?? items[0];
}

function getDefaultItemValue<T>(items: SelectOption<T>[]): T {
    return getDefaultItem(items).value;
}

export interface SelectProps<T extends PropertyKey> extends PropsWithChildren {
    items: SelectOption<T>[];
    selectedValue?: NoInfer<T>;
    defaultValue?: NoInfer<T>;
    customChildren?: boolean;
    onChange?: (selection: NoInfer<T>) => void;
    closeOnSelect?: boolean;
    className?: string;
    scrollAreaClassName?: string;
    open?: boolean;
    onOpenChange?: (open: boolean) => void;
}

export function Select<T extends PropertyKey>({
    items,
    customChildren = false,
    defaultValue = getDefaultItemValue(items),
    onChange,
    selectedValue,
    children,
    closeOnSelect = true,
    className,
    scrollAreaClassName,
    open: _open,
    onOpenChange,
}: SelectProps<T>) {
    const [selectedItem, setSelectedItem] = useControlledState({
        initialValue: defaultValue,
        debugName: "Select",
        managedValue: selectedValue,
        handleChange: onChange,
    });

    // const isOpenManaged = _open !== undefined;
    const [open, setOpen] = useControlledState({
        initialValue: false,
        debugName: "Select",
        managedValue: _open,
        handleChange: onOpenChange,
    });

    const ref = useRef<HTMLDivElement>(null);
    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (open && menuRef.current) {
            menuRef.current.focus();
        }
    }, [open]);

    function getItem(value: T) {
        return items.find((i) => i.value === value);
    }

    const rotation = useSpringValue(0);

    useEffect(() => {
        rotation.start(open ? 180 : 0);
    }, [open, rotation]);

    let currentLabel = getItem(selectedItem)?.label;

    if (typeof currentLabel === "string") {
        currentLabel = (
            <Text size="md">
                {currentLabel}
            </Text>
        );
    }

    return (
        <>
            <div
                ref={ref}
                className="relative"
            >
                <Clickable
                    className={cn(
                        styles.select,
                        className,
                        border.interactive,
                        border.autofocus,
                        border.animate,
                        open && border.focused,
                    )}
                    onClick={(e) => {
                        if (e.detail > 1) {
                            e.preventDefault();
                        }
                        setOpen((o) => !o);
                    }}
                >
                    { customChildren
                        ? children
                        : (
                            <>
                                <div className="flex-1/1">
                                    {currentLabel}
                                </div>
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
                            </>
                        )}
                </Clickable>
                <div className={cn("absolute top-6/5 left-0 w-full", z.select)}>
                    {open && (
                        <div
                            onBlur={({ relatedTarget }) => {
                                invariant(ref.current, "how are we running this without ref.current being set");
                                if (ref.current.contains(relatedTarget)) {
                                    return;
                                }
                                setOpen(false);
                            }}
                            ref={menuRef}
                            tabIndex={-1}
                        >
                            <SelectMenu
                                items={items}
                                scrollAreaClassName={scrollAreaClassName}
                                selectedItem={selectedItem}
                                onChange={(label) => {
                                    setSelectedItem(label);
                                    if (closeOnSelect) {
                                        setOpen(false);
                                    }
                                }}
                            />
                        </div>
                    )}
                </div>
            </div>
        </>
    );
}

export function SearchableSelect<T extends PropertyKey>({ onChange, ...props }: SelectProps<T>) {
    const [open, _setOpen] = useState(false);

    return (
        <Select
            open={open}
            {...props}
            onChange={(value) => {
                onChange?.(value);
            }}
            customChildren
            className={cn(border.directDisable, "p-0")}
        >
            <Input
                onChange={() => { }}
            />
        </Select>
    );
}
