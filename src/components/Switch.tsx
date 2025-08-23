import cn from "@/utils/cn";
import error from "@/utils/error";
import { animated, useSpring } from "@react-spring/web";

import { Clickable } from "./Clickable";

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

export interface SwitchProps {
    initialValue?: boolean;
    onChange?: (value: boolean) => void;
}

export function Switch({ initialValue = false, onChange }: SwitchProps) {
    const [enabled, setEnabled] = useState(initialValue);
    const [state, setState] = useState(initialValue ? SwitchState.ON : SwitchState.OFF);

    const [{ cx }, switchPosApi] = useSpring(() => ({
        cx: xFromSwitchState(state),
        config: {
            mass: 0.5,
        },
    }));

    useEffect(() => {
        switchPosApi.start({
            cx: xFromSwitchState(state),
        });
    }, [state, switchPosApi]);

    return (
        <Clickable
            className={cn("w-12 h-8 rounded-full flex justify-center transition-colors duration-250", enabled ? "bg-primary" : "bg-bg-300")}
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
                if (enabled) {
                    setState(SwitchState.OFF);
                } else {
                    setState(SwitchState.ON);
                }
                onChange?.(!enabled);
                setEnabled(!enabled);
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
                    r={8}
                    className={cn("transition-colors duration-250", enabled ? "fill-neutral" : "fill-bg-fg-600")}
                />
            </svg>
        </Clickable>
    );
}

