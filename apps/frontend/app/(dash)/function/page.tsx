import dynamic from "next/dynamic";
const Ide = dynamic(() => import("./Ide.tsx"));
export default function Page() { return <Ide />; }
