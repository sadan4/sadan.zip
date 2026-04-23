import { Link } from "@/components/Links";
import { Text } from "@/components/Text";
import { rootRouteId, useMatch } from "@tanstack/react-router";

const quotes = Object.freeze([
    "Get away from there! I'm expecting an important message!",
    "Quick! It's about to go critical!",
    "What the hell is going on with our equipment?",
    "It wasn't meant to do this in the first place!",
    "I'm afraid we'll be deviating a bit from standard analysis procedures today.",
    "Now, now, if you follow standard insertion procedures, everything will be fine.",
    "Well, go ahead. Let's let him in now.",
    "Testing...testing... Everything seems to be in order.",
    "Very good. We'll take it from here.",
    "Uh...it's probably not a problem...probably...but I'm showing a small discrepancy in...well, no, it's well within acceptable bounds again. Sustaining sequence.",
    "Wha-what's he doing in there?",
    "Get him out of there! Shut down the equipment and someone get him out!",
    "Shutting down. Attempting shut down. It's not...it's-it's not...it's not shutting down...it's not...",
    "Why didn't they listen!",
    "Oh my God...we're doomed!",
    "Don't shoot! I'm with the science team!",
    "This is my hiding spot, and I'm not moving until the situation is drastically improved. Now go away and don't tell anyone I'm here.",
    "Put that down—it's a prototype.",
    "It's much too unpredictable. Don't let it overcharge!",
    "It's ready! You must go! Now!",
]);

const [fallbackQuote] = quotes;

function getSeededQuote(seed: number | undefined): string {
    if (typeof seed !== "number") {
        return fallbackQuote;
    }

    return quotes[seed % quotes.length] ?? fallbackQuote;
}

export function NotFoundPage() {
    const rootMatch = useMatch({ from: rootRouteId });
    const quote = getSeededQuote(rootMatch.loaderData?.notFoundQuoteSeed);

    return (
        <div className="flex min-h-dvh flex-col items-center justify-center gap-4 px-4 text-center">
            <Text
                tag="p"
                size="2xl"
                weight="extraBold"
                color="accent"
                className="sm:max-w-3/4 sm:text-3xl"
            >
                {quote}
            </Text>
            <Text
                tag="p"
                size="lg"
                color="white-600"
            >
                This page doesn't exist...
            </Text>
            <Link
                to="/"
                className="rounded-md bg-accent-300 px-4 py-2 font-medium text-bg-300 hover:opacity-90"
            >
                Go back home
            </Link>
        </div>
    );
}
