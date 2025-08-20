import Cursor from "@/components/Cursor";
import BoundingCursor from "@/components/Cursor/BoundingCursor";
import { DotCursor } from "@/components/Cursor/DotCursor";
import cursorStyles from "@/components/Cursor/styles.module.css";
import ModalRenderRoot from "@/components/modal/ModalRenderRoot";
import cn from "@/utils/cn";

export function Boilerplate() {
    return (
        <>
            <Cursor>
                <DotCursor
                    className={cn("bg-bg-fg mix-blend-exclusion", cursorStyles.cursorZ)}
                    radius={10}
                    invert
                    lineOnText
                />
            </Cursor>
            <Cursor>
                <BoundingCursor
                    className={cn("bg-bg-fg mix-blend-exclusion", cursorStyles.cursorZ)}
                    frameLength={{
                        type: "dynamic",
                        factor: 1 / 10,
                        min: 8,
                        max: 25,
                    }}
                    unHoveredRadius={15}
                    thickness={4}
                    borderFocusedItems
                />
            </Cursor>
            <ModalRenderRoot />
        </>
    );
}
