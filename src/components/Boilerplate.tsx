import { useLoaderData } from "@/main";
import { z } from "@/styles";
import cn from "@/utils/cn";
import Cursor from "@components/Cursor";
import BoundingCursor from "@components/Cursor/BoundingCursor";
import { DotCursor } from "@components/Cursor/DotCursor";
import ModalRenderRoot from "@components/modal/ModalRenderRoot";

import { CustomCursorContext } from "./Cursor/context";

import { useContext, useEffect } from "react";

export interface BoilerplateProps {
    noCursor?: boolean;
}

export function Boilerplate({ noCursor }: BoilerplateProps) {
    const cursorContext = useContext(CustomCursorContext);
    const loaderData = useLoaderData();
    const gifBg = !loaderData?.config?.solidBg;

    useEffect(() => {
        if (gifBg) {
            document.body.classList.add("snow");
            return () => {
                document.body.classList.remove("snow");
            };
        }
    }, [gifBg]);

    noCursor ??= cursorContext;
    noCursor ??= loaderData?.config?.noCursor;
    return (
        <>
            {
                !noCursor && (
                    <>
                        <Cursor>
                            <DotCursor
                                className={cn("bg-fg-500 mix-blend-exclusion", z.cursor)}
                                radius={10}
                                invert
                                lineOnText
                            />
                        </Cursor>
                        <Cursor>
                            <BoundingCursor
                                className={cn("bg-fg-500 mix-blend-exclusion", z.cursor)}
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
