import native from "./native";

function handleBuild(buildHash: string) {
    console.log("Handling build", { buildHash });
}

native.start(handleBuild);
