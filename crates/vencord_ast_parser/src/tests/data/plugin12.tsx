import { ApplicationCommandInputType, ApplicationCommandOptionType, findOption, sendBotMessage } from "@api/Commands";
import { definePluginSettings } from "@api/Settings";
import { Devs } from "@utils/constants";
import definePlugin, { OptionType } from "@utils/types";
import { FluxDispatcher, UserStore } from "@webpack/common";

const settings = definePluginSettings({
    autoFillArguments: {
        description: "Automatically fill command with all arguements instead of just required ones",
        type: OptionType.BOOLEAN,
        default: true,
    },
    allowNewlinesInCommands: {
        description: "Allow newlines in command inputs (CTRL + Shift + Enter)",
        type: OptionType.BOOLEAN,
        default: true,
    }
});

function fetchIndex(target: object) {
    FluxDispatcher.dispatch({
        type: "APPLICATION_COMMAND_INDEX_FETCH_REQUEST",
        target
    });
}

export default definePlugin({
    name: "BetterCommands",
    description: "Enhances the command system with miscellaneous improvements.",
    dependencies: ["CommandsAPI"],
    tags: ["Appearance", "Commands", "Shortcuts"],
    authors: [Devs.thororen],
    settings,
    patches: [
        {
            find: '"italics"),!0;',
            predicate: () => settings.store.allowNewlinesInCommands,
            replacement: [
                {
                    match: /case (\i\.\i)\.TAB:if\(null!=(\i).selection&&\i\((\i)(?=.{0,300}(\i\.\i\.insertText))/,
                    replace: (orig, keys, editor, event, insertText) => {
                        return `case ${keys}.ENTER:
                                    if(${event}.shiftKey && ${event}.ctrlKey){
                                        ${event}.preventDefault();
                                        ${event}.stopPropagation();
                                        ${insertText}(${editor},'\\n');
                                        return true;
                                    }
                                    break;
                                ${orig}`;
                    }
                }
            ]
        }
    ],
});

