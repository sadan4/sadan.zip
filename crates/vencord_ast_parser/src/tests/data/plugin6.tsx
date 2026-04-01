import definePlugin from "@utils/types";

export default definePlugin({
    name: "Plugin6",
    patches: [
        {
            find: 'tutorialId:"instant-invite"',
            replacement: [
                // Render null instead of the buttons if the channel is hidden
                ...[
                    "renderEditButton",
                    "renderInviteButton",
                ].map(func => ({
                    match: new RegExp(`(?<=${func}\\(\\){)`, "g"), // Global because Discord has multiple declarations of the same functions
                    replace: "if($self.isHiddenChannel(this.props.channel))return null;"
                }))
            ]
        },

    ],
});
