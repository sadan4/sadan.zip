import type { SpringConfig } from "@react-spring/web";

export function borderHoldAnimConfig(held: boolean) {
    return (k: "opacity" | "progress"): SpringConfig => {
        switch (k) {
            case "opacity":
                return {};
            case "progress":
                return {
                    mass: 5,
                    friction: held ? 75 : 50,
                };
        }
    };
}
