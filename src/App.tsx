import Avatar from "./components/Avatar";
import { DefaultFooter, FooterContainer } from "./components/Footer";
import { DiscordIconLink, FortniteDBIconLink, GithubIconLink, LastFMIconLink, NameMCIconLink, SteamIconLink } from "./components/Links";
import Name from "./components/Name";

function Links() {
  return <div className="flex [&_svg]:h-14 gap-3 [&_svg]:text-bg-fg-600">
    <DiscordIconLink userId="521819891141967883" />
    <NameMCIconLink UUID="b7c4f5b1-762f-41ea-b6b4-45aba74198e5" />
    <LastFMIconLink username="sadan4" />
    <SteamIconLink userId="sadan4" />
    <FortniteDBIconLink username="sadan4" />
    <GithubIconLink username="sadan4" />
  </div>;
}

export default function App() {
  return (
    <>
      <FooterContainer
        footer={<DefaultFooter />}
        className="flex justify-center"
      >
        <div className="mt-52 flex items-center flex-col">
          <Avatar className="rounded-full w-52" />
          <Name />
          <Links />
          <div className="text-success mt-6">
            Random loser on the internet.
          </div>
        </div>
      </FooterContainer>
    </>
  );
}
