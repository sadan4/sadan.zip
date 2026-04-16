import definePlugin from "@utils/types";

export default definePlugin({
    name: "Plugin2",
    patches: [
        {
            find: ".handleSendMessage,onResize:",
            replacement: {
                // https://regex101.com/r/7iswuk/1
                match: /let (\i)=\i\.\i\.parse\((\i),.+?\.getSendMessageOptions\(\{.+?\}\)?;(?=.+?(\i)\.flags=)(?<=\)\(({.+?})\)\.then.+?)/,
                replace: (m, parsedMessage, channel, replyOptions, extra) => m +
                    `if(await Vencord.Api.MessageEvents._handlePreSend(${channel}.id,${parsedMessage},${extra},${replyOptions}))` +
                    "return{shouldClear:false,shouldRefocus:true};"
            }
        },
    ]
});
