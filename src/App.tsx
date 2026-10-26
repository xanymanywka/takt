import { useEffect } from 'react';
import { Lead } from './components/Lead';
import { Preview } from './components/Preview';
import { Sidebar } from './components/Sidebar';
import { UpdateBanner } from './components/UpdateBanner';
import { About } from './screens/About';
import { Chat } from './screens/Chat';
import { Code } from './screens/Code';
import { Models } from './screens/Models';
import { Orchestra } from './screens/Orchestra';
import { Onboarding } from './screens/Onboarding';
import { Settings } from './screens/Settings';
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
      <UpdateBanner />

      {screen === 'start' && <Start />}
      {screen === 'about' && <About />}
      {screen === 'models' && <Models />}
      {screen === 'settings' && <Settings />}
      {screen === 'onboarding' && <Onboarding />}
      {screen === 'chat' && (
        <div className="win" id="chat">
          <Sidebar />
          {tab === 'chat' && <Chat />}
          {tab === 'code' && <Code />}
          {tab === 'orchestra' && <Orchestra />}
          {tab === 'chat' && <Preview />}
        </div>
      )}
    </div>
  );
}

export default App;
