/**
 * @type {HTMLElement}
    */
let myname = document.getElementById("name")
let names = [
    "sadan",
    ":3",
    "hiiiii",
    "minecraft addict",
    "save the world player",
    "linux user",
    "WOMP WOMP",
    "avid ozone fan",
    "i use NixOS, btw"
]
let typing = false;
const randomName = () => names.filter((n) => myname.innerHTML !== n)[Math.floor(Math.random() * (names.length-1))];
document.getElementById("name").addEventListener("click", async (e) => {
    if (typing) return;
    typing = true;
    await typewriter(randomName(), myname);
    typing  = false;
})
/**
 * @param text {string}
 * @param e {HTMLElement}
 */
async function typewriter(text, e){
    const cur = e.innerHTML;
    for(let i = cur.length; i >= 0; i--){
        e.innerHTML = cur.substring(0, i) + "|";
        await new Promise(r => setTimeout(r, 50));
    }
    e.innerHTML = "|";
    for(let i = 0; i < text.length; i++){
        e.innerHTML = text.substring(0, i) + "|";
        await new Promise(r => setTimeout(r, 50));
    }
    e.innerHTML = text;
}
