export function discordUrl(userId: string): URL {
    return new URL(`https://discord.com/users/${userId}`);
}
