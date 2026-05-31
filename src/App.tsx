import { Island } from "./components/Island";

export const isTauri = "__TAURI_INTERNALS__" in window;

function App() {
  if (isTauri) {
    // In Tauri: island fills the entire window — window size = island size.
    // No centering, no wrapper. The window IS the island.
    return <Island />;
  }

  // Browser preview only
  return (
    <div style={{
      width: "100vw", height: "100vh",
      display: "flex", alignItems: "flex-start", justifyContent: "center",
      paddingTop: 40,
      background: "linear-gradient(135deg, #0a0d1a 0%, #111428 50%, #0c0e1e 100%)",
    }}>
      <div style={{
        position: "fixed", bottom: 20, left: "50%", transform: "translateX(-50%)",
        color: "rgba(100,130,200,0.4)", fontSize: 11, fontFamily: "system-ui",
      }}>
        PREVIEW — hover · click · long-press para settings
      </div>
      <Island />
    </div>
  );
}

export default App;
