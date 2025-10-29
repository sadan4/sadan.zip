import { ScrollArea } from "@/components/layout/ScrollArea";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { friends } from "@/utils/friends";

import { FriendButton } from "./shared";

export function FriendModalMobile() {
    return (
        <div className="fixed inset-x-1/5 inset-y-1/8 bg-bg-100/25 sb-track-bg-100/25">
            <ScrollArea className="h-full max-h-full">
                <div className="m-2 grid grid-flow-row-dense grid-cols-[repeat(auto-fill,--spacing(24))] justify-center gap-4">
                    {friends.map((friend) => {
                        return (
                            <FriendButton
                                friend={friend}
                                tooltipPosition={TooltipPosition.TOP}
                                key={friend.name}
                                mobile
                            />
                        );
                    })}
                </div>
            </ScrollArea>

        </div>
    );
}
