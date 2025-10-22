import dynamic from "next/dynamic";
// const Lambda = dynamic(() => import("./Lambda.tsx"));
const Ide = dynamic(() => import("./Ide"));
// export default function Page() { return <Lambda />; }
export default function Page() { return <Ide />; }
