import cn from "@/utils/cn";
import error from "@/utils/error";
import { animated, useSpringValue } from "@react-spring/web";

import { Clickable } from "./Clickable";

import invariant from "invariant";
import { useEffect, useState } from "react";

enum SwitchState {
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
    initialValue?: boolean;
    value?: boolean;
    onChange?: (value: boolean) => void;
}

export function Switch({ initialValue, value, onChange }: SwitchProps) {
    invariant(!(initialValue !== undefined && value !== undefined), "Switch cannot be both controlled and uncontrolled");

    const isManaged = value !== undefined;
    const [internalEnabled, setInternalEnabled] = useState(value ?? initialValue ?? false);
    const enabled = isManaged ? value : internalEnabled;
    const [state, setState] = useState(initialValue ? SwitchState.ON : SwitchState.OFF);

    const cx = useSpringValue(xFromSwitchState(state), {
        config: {
            mass: 0.5,
        },
    });

    const r = useSpringValue(rFromSwitchState(state), {
        config: {
            mass: 0.1,
        },
    });

    useEffect(() => {
        if (isManaged) {
            setInternalEnabled(value);
            setState(value ? SwitchState.ON : SwitchState.OFF);
        }
    }, [isManaged, value]);

    useEffect(() => {
        cx.start(xFromSwitchState(state));
        r.start(rFromSwitchState(state));
    }, [state, cx, r]);

    return (
        <Clickable
            className={cn("w-12 h-8 rounded-full flex justify-center transition-[background-color,border-radius] duration-250", enabled ? "bg-primary" : " border border-bg-fg-800 bg-bg-300")}
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
                className="flex items-center justify-center w-full h-full"
            >
                <animated.circle
                    cx={cx}
                    cy={12}
                    r={r}
                    className={cn("transition-colors duration-250", enabled ? "fill-neutral" : "fill-bg-fg-600")}
                />
            </svg>
        </Clickable>
    );
}

