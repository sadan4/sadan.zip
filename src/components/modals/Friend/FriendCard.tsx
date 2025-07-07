import { Clickable } from "@/components/Clickable";
import Discord from "@/components/icons/Discord";
import LinkIcon from "@/components/icons/FriendLink";
import Circle from "@/components/layout/Circle";
import { Square } from "@/components/testing";
import { discordUrl } from "@/utils/constants";
import type { Friend } from "@/utils/friends";


export interface FriendCardProps {
    friend: Friend;
}

export default function FriendCard({ friend }: FriendCardProps) {
    return (
        <Circle
            radius={192}
            numItems={4}
        >
            {
                friend.url
                    ? (
                        <Clickable
                            tag="a"
                            target="_blank"
                            rel="noopener noreferrer"
                            className="w-12 h-12 flex items-center justify-center bg-bg-100 rounded-full"
                        >
                            <LinkIcon className="w-9 h-9" />
                        </Clickable>
                    )
                    : (
                        <Clickable
                            tag="a"
                            className="w-12 h-12 flex items-center justify-center bg-bg-100 rounded-full cursor-not-allowed brightness-50"
                            onClick={(e) => {
                                e.preventDefault();
                            }}
                        >
                            <LinkIcon className="w-9 h-9" />
                        </Clickable>
                    )
            }
            <div className="text-info-500 text-3xl">
                {friend.name}
            </div>
            {
                friend.discordId
                    ? (
                        <Clickable
                            tag="a"
                            href={discordUrl(friend.discordId)
                                .toString()}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="w-12 h-12 flex items-center justify-center bg-bg-100 rounded-full"
                        >
                            <Discord className="w-8 h-8" />
                        </Clickable>
                    )
                    : (
                        <>
                            <Square />
                        </>
                    )
            }
        </Circle>
    );
}
