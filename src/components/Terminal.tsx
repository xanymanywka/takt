import React, { useEffect, useRef } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { isTauriEnv, listen, pty_close, pty_open, pty_resize, pty_write } from '../lib/ipc';
import { useStore } from '../store';

interface TerminalProps {
  worktree: string;
}

export const Terminal: React.FC<TerminalProps> = ({ worktree }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const theme = useStore((state) => state.theme);

  useEffect(() => {
    if (!containerRef.current) return;

    const isDark = theme === 'dark';
    const term = new XTerm({
      cursorBlink: true,
      fontSize: 13.5,
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, monospace',
      theme: isDark
        ? {
            background: '#151616',
            foreground: '#f4f5f0',
            cursor: '#c8ff00',
            selectionBackground: 'rgba(200, 255, 0, 0.35)',
            black: '#080808',
            brightBlack: '#242626',
            green: '#7ed69a',
            cyan: '#6fd3c3',
            blue: '#8ea6ff',
            yellow: '#ffb648',
          }
        : {
            background: '#ffffff',
            foreground: '#0b0d0c',
            cursor: '#3f5a00',
            selectionBackground: 'rgba(200, 255, 0, 0.45)',
            black: '#e6e9e0',
            brightBlack: '#d9ded0',
            green: '#1f7a3d',
            cyan: '#0f7c6d',
            blue: '#2f4fd0',
            yellow: '#8f4d00',
          },
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);
    fitAddon.fit();

    let ptyId = '';
    let isDisposed = false;
    let unlistenData: (() => void) | null = null;

    const promptStr = `\x1b[38;2;111;211;195m${worktree}\x1b[0m % `;

    if (isTauriEnv()) {
      pty_open(`~/orchestra/orchestra/${worktree}`, term.cols, term.rows)
        .then(async (id) => {
          if (isDisposed) {
            pty_close(id);
            return;
          }
          ptyId = id;
          unlistenData = await listen<{ id: string; data: string }>('pty://data', (e) => {
            if (e.payload.id === ptyId) {
              term.write(e.payload.data);
            }
          });
        })
        .catch((err) => {
          term.write(`\r\nОшибка открытия PTY: ${String(err)}\r\n`);
        });

      term.onData((data) => {
        if (ptyId) {
          pty_write(ptyId, data);
        }
      });
    } else {
      // Мок-терминал с эхо
      term.write(
        `\x1b[38;2;179;230;0m[такт]\x1b[0m подключение к ворктри \x1b[1m${worktree}\x1b[0m (эхо-терминал)\r\n`
      );
      term.write(promptStr);

      let lineBuffer = '';

      term.onData((data) => {
        for (let i = 0; i < data.length; i++) {
          const char = data[i];
          if (char === '\r') {
            term.write('\r\n');
            const cmd = lineBuffer.trim();
            if (cmd === 'clear') {
              term.clear();
            } else if (cmd === 'pwd') {
              term.write(`~/orchestra/orchestra/${worktree}\r\n`);
            } else if (cmd === 'ls') {
              term.write(`Cargo.toml  README.md  docs  src  src-tauri\r\n`);
            } else if (cmd.length > 0) {
              term.write(`[echo] ${cmd}\r\n`);
            }
            lineBuffer = '';
            term.write(promptStr);
          } else if (char === '\u007F') {
            // Backspace
            if (lineBuffer.length > 0) {
              lineBuffer = lineBuffer.slice(0, -1);
              term.write('\b \b');
            }
          } else if (char >= ' ') {
            lineBuffer += char;
            term.write(char);
          }
        }
      });
    }

    const handleResize = () => {
      try {
        fitAddon.fit();
        if (ptyId && isTauriEnv()) {
          pty_resize(ptyId, term.cols, term.rows);
        }
      } catch (e) {}
    };

    window.addEventListener('resize', handleResize);

    return () => {
      isDisposed = true;
      window.removeEventListener('resize', handleResize);
      if (unlistenData) unlistenData();
      if (ptyId && isTauriEnv()) {
        pty_close(ptyId);
      }
      term.dispose();
    };
  }, [worktree, theme]);

  return <div ref={containerRef} className="term-body" />;
};
