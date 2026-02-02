import { useMediaQuery } from "@/hooks/mediaQuery";
import { ClientOnly } from "@tanstack/react-router";

import { FriendModalMobile } from "./mobile";
import { FriendModalNormal } from "./normal";


export function FriendModal() {
    const isDesktopScreen = useMediaQuery("(width >= 735px)");

    return (
        <ClientOnly>
            {isDesktopScreen ? <FriendModalNormal /> : <FriendModalMobile />}
        </ClientOnly>
    );
}

