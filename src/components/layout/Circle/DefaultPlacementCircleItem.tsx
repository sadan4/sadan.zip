// this has its own file for react hot reload, as it's found with an equality compare
// react hot reload would replace this with an exact copy, with a new identity, breaking the comparison

import { CircleItemContext } from "./context";
import * as styles from "./styles.module.scss";

import { type PropsWithChildren, use } from "react";

export function DefaultPlacementCircleItem({ children }: PropsWithChildren) {
    const { x: left, y: top } = use(CircleItemContext);

    return (
        <div
            className={styles.default}
            style={{
                top,
                left,
            }}
        >
            {children}
        </div>
    );
}

