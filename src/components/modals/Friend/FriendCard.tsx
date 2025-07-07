import Circle from "@/components/layout/Circle";
import { Square } from "@/components/testing";
import type { Friend } from "@/utils/friends";

export interface FriendCardProps {
    friend: Friend;
}
export default function FriendCard({ friend: _ }: FriendCardProps) {
    return (
        <Circle
            radius={192}
            numItems={3}
            offset={-1}
        >
            <Square />
            {/* {friend.url
              && <FriendWebsiteLink href={friend.url.toString()} />} */}
            <Square />
            <Square />
        </Circle>
    );
}
