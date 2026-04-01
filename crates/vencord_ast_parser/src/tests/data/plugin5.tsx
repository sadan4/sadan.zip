import definePlugin from "@utils/types";

const CONNECT = 1n << 20n;

export default definePlugin({
    name: "Plugin5",
    patches: [
        {
            find: "#{intl::ROLE_REQUIRED_SINGLE_USER_MESSAGE}",
            replacement: [
                {
                    // Change the permissionOverwrite check to CONNECT if the channel is locked
                    match: /permissionOverwrites\[.+?\i=(?<=context:(\i)}.+?)(?=(.+?)VIEW_CHANNEL)/,
                    replace: (m, channel, permCheck) => `${m}!Vencord.Webpack.Common.PermissionStore.can(${CONNECT}n,${channel})?${permCheck}CONNECT):`
                },
            ]
        },
    ],
});
