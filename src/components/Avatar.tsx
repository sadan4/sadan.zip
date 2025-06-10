import avatar from "@/assets/avatar.webp";

export default function Avatar() {
  return (
    <img
      className="rounded-full max-w-sm max-h-max w-auto h-auto"
      src={avatar}
      alt="my discord profile picture, imagine a cute cat!"
    />
  );
}
