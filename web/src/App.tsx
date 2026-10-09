import { Shell } from "./Shell";
import { fixtureThreads } from "./fixture";

export function App() {
  return <Shell threads={fixtureThreads} />;
}
