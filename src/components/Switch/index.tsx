import cn from "@/utils/cn";
import error from "@/utils/error";
import { animated, useSpring } from "@react-spring/web";

import styles from "./styles.module.css";
import { Clickable } from "../Clickable";
import { type StandardTextProps, Text } from "../Text";

import invariant from "invariant";
import { type ComponentPropsWithRef, useEffect, useState } from "react";

const enum SwitchState {
    OFF,
    ON,
    HELD,
}

function xFromSwitchState(state: SwitchState): number {
    switch (state) {
        case SwitchState.OFF:
            return 6;
        case SwitchState.ON:
            return 18;
        case SwitchState.HELD:
            return 12;
        default: {
            error("unhandled state");
        }
    }
}

function rFromSwitchState(state: SwitchState) {
    switch (state) {
        case SwitchState.OFF:
            return 8;
        case SwitchState.ON:
            return 8;
        case SwitchState.HELD:
            return 7;
        default: {
            error("unhandled state");
        }
    }
}

export interface SwitchProps {
    /**
     * the initial value of the switch if uncontrolled
     */
    initialValue?: boolean;
    /**
     * the controlled value of the switch
     */
    value?: boolean;
    /**
     * Called when the switch is toggled.
     */
    onChange?: (value: boolean) => void;
}

/**
 * An on-off switch with animations.
 */
export function Switch({ initialValue, value, onChange }: SwitchProps) {
    invariant(!(initialValue !== undefined && value !== undefined), "Switch cannot be both controlled and uncontrolled");

    const isManaged = value !== undefined;
    const [internalEnabled, setInternalEnabled] = useState(value ?? initialValue ?? false);
    const enabled = isManaged ? value : internalEnabled;
    const [state, setState] = useState(initialValue ? SwitchState.ON : SwitchState.OFF);

    const { cx, r } = useSpring({
        cx: xFromSwitchState(state),
        r: rFromSwitchState(state),
    });

    useEffect(() => {
        if (isManaged) {
            setInternalEnabled(value);
            setState(value ? SwitchState.ON : SwitchState.OFF);
        }
    }, [isManaged, value]);

    return (
        <Clickable
            className={cn(styles.switch, enabled && styles.enabled)}
            onMouseDown={(e) => {
                // stop random other text from being selected
                if (e.detail > 1) {
                    e.preventDefault();
                }
                setState(SwitchState.HELD);
            }}
            onMouseUp={() => {
                if (state !== SwitchState.HELD) {
                    return;
                }
                if (!isManaged) {
                    if (enabled) {
                        setState(SwitchState.OFF);
                    } else {
                        setState(SwitchState.ON);
                    }
                } else {
                    setState(enabled ? SwitchState.ON : SwitchState.OFF);
                }
                onChange?.(!enabled);
                setInternalEnabled(!enabled);
            }}
            onMouseLeave={() => {
                if (enabled) {
                    setState(SwitchState.ON);
                } else {
                    setState(SwitchState.OFF);
                }
            }}
        >
            <svg
                viewBox="0 0 24 24"
            >
                <animated.circle
                    cx={cx}
                    cy={12}
                    r={r}
                    className={styles.thumb}
                />
            </svg>
        </Clickable>
    );
}

export interface LabeledSwitchProps extends SwitchProps, Omit<ComponentPropsWithRef<"div">, "onChange" | "color">, Omit<StandardTextProps, "center"> {
}
export function LabeledSwitch({
    initialValue,
    className,
    children,
    value,
    onChange,
    noselect = true,
    nowrap,
    size = "md",
    weight,
    color,
    ...props
}: LabeledSwitchProps) {
    return (
        <div
            className={cn("inline-flex items-center gap-3", className)}
            {...props}
        >
            <div className="flex-[1_1_0]">
                <Text
                    noselect={noselect}
                    nowrap={nowrap}
                    size={size}
                    weight={weight}
                    color={color}
                >
                    {children}
                </Text>
            </div>
            <Switch
                initialValue={initialValue}
                value={value}
                onChange={onChange}
            />
        </div>
    );
}

