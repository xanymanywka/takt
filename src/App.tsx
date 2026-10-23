import { useEffect } from 'react';
import { Lead } from './components/Lead';
import { Sidebar } from './components/Sidebar';
import { Chat } from './screens/Chat';
import { Code } from './screens/Code';
import { Orchestra } from './screens/Orchestra';
import { Start } from './screens/Start';
import { useStore } from './store';

function App() {
  const screen = useStore((state) => state.screen);
  const tab = useStore((state) => state.tab);
  const init = useStore((state) => state.init);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let cancelled = false;

    init()
      .then((cl) => {
        if (cancelled) {
          if (cl) cl();
        } else {
          cleanup = cl;
        }
      })
      .catch((err) => {
        console.error('Store init failed', err);
      });

    return () => {
      cancelled = true;
      if (cleanup) cleanup();
    };
  }, [init]);

  return (
    <div className="page">
      <Lead />

      {screen === 'start' ? (
        <Start />
      ) : (
        <div className="win" id="chat">
          <Sidebar />
          {tab === 'chat' && <Chat />}
          {tab === 'code' && <Code />}
          {tab === 'orchestra' && <Orchestra />}
        </div>
      )}
    </div>
  );
}

export default App;
