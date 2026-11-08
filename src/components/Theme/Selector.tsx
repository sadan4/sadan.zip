import { cn } from "@/utils/cn";
import { isNumber } from "@/utils/functional";
import { TextmateTheme, themeDisplayNames } from "@/utils/textmate/theme";

import { ThemeContext } from "./misc";
import { Select, type SelectOption } from "../Select";

import { use } from "react";

export function ThemeSelector() {
    const cx = use(ThemeContext);

    return (
        <Select
            selectedValue={cx.theme}
            onChange={(newTheme) => {
                cx.updateTheme(newTheme);
            }}
            items={Object
                .values(TextmateTheme)
                .filter(isNumber)
                .map((theme: TextmateTheme) => {
                    const name = themeDisplayNames[theme];

                    return {
                        label: name,
                        typedValue: name,
                        value: theme,
                        key: theme,
                    } satisfies SelectOption<TextmateTheme>;
                })}
            className="w-48"
            scrollAreaClassName={cn("max-h-[25vh]")}
        />
    );
}
