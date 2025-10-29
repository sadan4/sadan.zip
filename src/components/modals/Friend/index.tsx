import { useMediaQuery } from "@/hooks/mediaQuery";

import { FriendModalMobile } from "./mobile";
import { FriendModalNormal } from "./normal";


export default function FriendModal() {
    const isNormalScreen = useMediaQuery("(width >= 735px)");

    if (isNormalScreen) {
        return <FriendModalNormal />;
    }
    return <FriendModalMobile />;
}

