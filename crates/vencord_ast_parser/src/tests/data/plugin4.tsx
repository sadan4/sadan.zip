import definePlugin from "@utils/types";

export default definePlugin({
    name: "Plugin4",
    patches: [
        {
            find: "Unexpected value for option",
            replacement: {
                match: /,(\i)\.execute\((\i),(\i)\)/,
                replace: (_, cmd, args, ctx) => `,Vencord.Api.Commands._handleCommand(${cmd}, ${args}, ${ctx})`
            }
        },
    ],
});
