import { Shell } from "./Shell";
import { fixtureBodies, fixtureThreads } from "./fixture";

export function App() {
  return <Shell threads={fixtureThreads} bodies={fixtureBodies} />;
}
