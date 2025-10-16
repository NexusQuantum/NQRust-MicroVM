import dynamic from "next/dynamic";
// const Lambda = dynamic(() => import("./Lambda.tsx"));
const Ide = dynamic(() => import("./Ide.tsx"));
export default function Page() { return <Ide />; }
