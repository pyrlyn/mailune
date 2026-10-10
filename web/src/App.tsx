import { Shell } from "./Shell";
import { fixtureAi, fixtureAssist } from "./ai";
import { fixtureBodies, fixtureThreads } from "./fixture";

export function App() {
  return <Shell threads={fixtureThreads} bodies={fixtureBodies} ai={fixtureAi} assist={fixtureAssist} />;
}
