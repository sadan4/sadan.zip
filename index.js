{
    const [block, unblock] = toggler("block");
    const [pointer, unpointer] = toggler("pointer");
    let typing = true;
    /**
     * @type {HTMLElement}
        */
    let myname = document.getElementById("name")
    addEventListener("DOMContentLoaded", () => {
        setTimeout(() => {
            typewriter(myname, "Click me!", 50).then(async () => {
                await new Promise(r => setTimeout(r, 750));
                await typewriter(myname, "sadan", 50);
                typing = false;
                unblock(myname);
                pointer(myname);
            });
        }, 1000);

    });
    let names = [
        "sadan",
        ":3",
        "hiiiii",
        "minecraft addict",
        "save the world player",
        "linux user",
        "WOMP WOMP",
        "avid ozone fan",
        "i use NixOS, btw",
        "Lazily Evaluated",
        "Reproducible",
        "Declarative",
        "Open Source",
        ":husk:",
        ":blobcatcozy:",
        ":wires:",
        "Hop on Vencord",
        ['<img src="https://cdn.discordapp.com/emojis/1026533070955872337.webp?size=128" alt=":blobcatcozy:" class="emoji"></img>', 35],
        ['<img src="/assets/creature.png" alt="creature" class="emoji"></img>', 35],
        ['<img src="https://cdn.discordapp.com/emojis/1026532993923293184.webp?size=128" alt=":husk:" class="emoji"></img>', 35],
        ['<img src="https://cdn.discordapp.com/emojis/1320236763494486087.webp?size=128" alt=":steamcatcozy:" class="emoji"></img>', 35],
        ['<img src="https://cdn.discordapp.com/emojis/1262562427422244874.webp?size=128" alt=":wires:" class="emoji"></img>', 35],
    ].map(x => Array.isArray(x) ? x : [x])
    const randomName = () => names.filter((n) => myname.innerHTML !== n)[Math.floor(Math.random() * (names.length - 1))];
    document.getElementById("name").addEventListener("click", async (e) => {
        if (typing) return;
        typing = true;
        unpointer(myname);
        block(myname);
        await typewriter(myname, ...randomName());
        unblock(myname);
        pointer(myname);
        typing = false;
    })
    let delDelay = 50;
    /**
     * @param text {string}
     * @param e {HTMLElement}
        */
    async function typewriter(e, text, delay = 50) {
        const cur = e.innerHTML;
        for (let i = cur.length; i >= 0; i--) {
            e.innerHTML = escapeHtml(cur.substring(0, i) + "|");
            await new Promise(r => setTimeout(r, delDelay));
        }
        e.innerHTML = "|";
        for (let i = 0; i < text.length; i++) {
            e.innerHTML = escapeHtml(text.substring(0, i) + "|");
            await new Promise(r => setTimeout(r, delay));
        }
        e.innerHTML = text;
        delDelay = Math.max(delay - 10, 0);
    }
    /**
     * @param {string} name className
     * 
     * @returns {[(e: HTMLElement) => void, (e: HTMLElement) => void]} a set of toggler functions
     */
    function toggler(name) {
        return [
            (e) => e.classList.add(name),
            (e) => e.classList.remove(name)
        ]
    }
    /*!
    * escape-html
    * Copyright(c) 2012-2013 TJ Holowaychuk
    * Copyright(c) 2015 Andreas Lubbe
    * Copyright(c) 2015 Tiancheng "Timothy" Gu
    * MIT Licensed
    */
    let matchHtmlRegExp = /["'&<>]/;
    function escapeHtml(string) {
        var str = '' + string
        var match = matchHtmlRegExp.exec(str)

        if (!match) {
            return str
        }

        var escape
        var html = ''
        var index = 0
        var lastIndex = 0

        for (index = match.index; index < str.length; index++) {
            switch (str.charCodeAt(index)) {
                case 34: // "
                    escape = '&quot;'
                    break
                case 38: // &
                    escape = '&amp;'
                    break
                case 39: // '
                    escape = '&#39;'
                    break
                case 60: // <
                    escape = '&lt;'
                    break
                case 62: // >
                    escape = '&gt;'
                    break
                default:
                    continue
            }

            if (lastIndex !== index) {
                html += str.substring(lastIndex, index)
            }

            lastIndex = index + 1
            html += escape
        }

        return lastIndex !== index
            ? html + str.substring(lastIndex, index)
            : html
    }
}