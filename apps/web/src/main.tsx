import { StrictMode, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Counter } from './Counter';
import { Setup } from './Setup';
import { ShareView } from './ShareView';
import './styles.css';

function App() {
  const [hash, setHash] = useState(location.hash);
  useEffect(() => {
    const onHash = () => setHash(location.hash);
    addEventListener('hashchange', onHash);
    return () => removeEventListener('hashchange', onHash);
  }, []);
  if (hash.startsWith('#/share?')) return <ShareView query={hash.slice('#/share?'.length)} />;
  return hash === '#/counter' ? <Counter /> : <Setup />;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
