import React, { useState } from 'react';
import { useT } from '../i18n';
import { useStore } from '../store';
import { PlusIcon, SendIcon } from './Icons';

export const Composer: React.FC = () => {
  const t = useT();
  const [text, setText] = useState('');
  const sendUserMessage = useStore((state) => state.sendUserMessage);
  const error = useStore((state) => state.error);

  const handleSend = () => {
    if (!text.trim()) return;
    sendUserMessage(text);
    setText('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      handleSend();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="composer-wrap">
      <div className="composer">
        <button
          type="button"
          className="plus"
          aria-label={t('composer.attach')}
          title={t('composer.attach')}
        >
          <PlusIcon size={16} />
        </button>

        <div className="in-wrap">
          <input
            type="text"
            className="in"
            placeholder={t('composer.placeholder')}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={handleKeyDown}
          />
        </div>

        <span className="hint">⌘ ↵</span>

        <button
          type="button"
          className="send"
          aria-label={t('composer.send')}
          title={t('composer.send')}
          onClick={handleSend}
        >
          <SendIcon size={16} />
        </button>
      </div>

      {error && <div className="error-banner">{error}</div>}
    </div>
  );
};
