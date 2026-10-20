import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { error } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
import { Language } from "@/utils/textmate";
import { useSuspenseQuery } from "@tanstack/react-query";

import { useModuleViewerStore } from "../-data";

import { useEffect, useMemo } from "react";

const PROVIDER_KEY = "discord-css-class-editor";
const URI = monaco.Uri.parse(`file:///${PROVIDER_KEY}.css`);

export function CssEditor() {
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);

    const { data: cssModules } = useSuspenseQuery({
        queryKey: ["get css modules", buildHash],
        queryFn() {
            return buildService.getCssModules();
        },
    });

    // FIXME: for now, it's easier to reason about
    // oxlint-disable-next-line react-x/no-unnecessary-use-memo
    const cssInfo: Monaco.languages.css.CSSDataV1 = useMemo(() => {
        const names = new Map<string, Monaco.languages.css.IPseudoClassData>();

        for (const module of cssModules) {
            for (const clazz of module.class_names) {
                const [name] = clazz.value.split(" ");

                if (names.has(name)) {
                    continue;
                }

                const data: Monaco.languages.css.IPseudoClassData = {
                    name: `.${name}`,
                    description: `defined in ${clazz.module}, exported as ${clazz.exported_as}`,
                };

                names.set(name, data);
            }
        }

        const pseudoElements = [...names.values()];

        return {
            version: 1.1,
            pseudoElements,
        };
    }, [cssModules]);

    useEffect(() => {
        const opts = monaco.languages.css.cssDefaults.options;

        // oxlint-disable-next-line logical-assignment-operators -- react compiler doesn't like it
        if (opts.data!.dataProviders == null) {
            opts.data!.dataProviders = {};
        }

        // oxlint-disable-next-line prefer-destructuring -- typescript bug?
        const dataProviders = opts.data!.dataProviders;

        if (PROVIDER_KEY in dataProviders) {
            error("our data provider is already set");
        }
        dataProviders[PROVIDER_KEY] = cssInfo;
        monaco.languages.css.cssDefaults.setOptions(opts);
        return () => void delete dataProviders[PROVIDER_KEY];
    }, [cssInfo]);

    return (
        <MonacoCodeEditor
            language={Language.CSS}
            uri={URI}
        />
    );
}
