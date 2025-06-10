import Avatar from "./components/Avatar";
import { DefaultFooter, FooterContainer } from "./components/Footer";
import Name from "./components/Name";

export default function App() {
  return (
    <>
      <FooterContainer
        footer={<DefaultFooter />}
        className="flex justify-center items-start content-center"
      >
        <div className="mt-32 flex items-center flex-col">
          <Avatar />
          <Name />
        </div>
      </FooterContainer>
    </>
  );
}
