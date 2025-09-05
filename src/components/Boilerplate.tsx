import cn from "@/utils/cn";
import Cursor from "@components/Cursor";
import BoundingCursor from "@components/Cursor/BoundingCursor";
import { DotCursor } from "@components/Cursor/DotCursor";
import ModalRenderRoot from "@components/modal/ModalRenderRoot";

import z from "./z";

export interface BoilerplateProps {
    noCursor?: boolean;
}

export function Boilerplate({ noCursor = false }: BoilerplateProps) {
    return (
        <>
            {
                !noCursor && (
                    <>
                        <Cursor>
                            <DotCursor
                                className={cn("bg-bg-fg mix-blend-exclusion", z.cursor)}
                                radius={10}
                                invert
                                lineOnText
                            />
                        </Cursor>
                        <Cursor>
                            <BoundingCursor
                                className={cn("bg-bg-fg mix-blend-exclusion", z.cursor)}
                                frameLength={{
                                    type: "dynamic",
                                    factor: 1 / 10,
                                    min: 8,
                                    max: 25,
                                }}
                                unHoveredRadius={15}
                                thickness={3}
                            />
                        </Cursor>
                    </>
                )
            }
            <ModalRenderRoot />
        </>
    );
}
