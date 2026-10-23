import React, { useState } from 'react';
import {
  ArrowRightIcon,
  CheckIcon,
  CircleTodoIcon,
  CodeRabbitIcon,
  DotIcon,
  GhIcon,
  ServerHostIcon,
  TerminalIcon,
  WranglerIcon,
} from '../components/Icons';
import { open_in_terminal, write_instructions } from '../lib/ipc';
import { useStore } from '../store';

export const Start: React.FC = () => {
  const setup = useStore((state) => state.setup);
  const updateChecksEnabled = useStore((state) => state.updateChecksEnabled);
  const updateCheckStatus = useStore((state) => state.updateCheckStatus);
  const updateCheckMessage = useStore((state) => state.updateCheckMessage);
  const checkUpdate = useStore((state) => state.checkUpdate);
  const setUpdateChecks = useStore((state) => state.setUpdateChecks);

  const [openSshRows, setOpenSshRows] = useState<Record<string, boolean>>({
    'nova-welcome-1': true,
  });

  const [instructionsCreated, setInstructionsCreated] = useState(false);

  const toggleSshRow = (host: string) => {
    setOpenSshRows((prev) => ({
      ...prev,
      [host]: !prev[host],
    }));
  };

  const handleCreateInstructions = async () => {
    try {
      await write_instructions();
      setInstructionsCreated(true);
    } catch (err) {
      console.error(err);
    }
  };

  const handleOpenTerminal = async () => {
    try {
      await open_in_terminal('~');
    } catch (err) {
      console.error(err);
    }
  };

  const sshHosts = setup?.ssh || [];
  const tools = setup?.tools || [];
  const mcpList = setup?.mcp || [];
  const mcpNames = mcpList.slice(0, 3).map((m) => m.name).join(', ') || 'context7, github, nova-crm';
  const mcpExtra = Math.max(0, mcpList.length - 3);

  const getToolIcon = (id: string) => {
    if (id === 'gh') return <GhIcon />;
    if (id === 'coderabbit') return <CodeRabbitIcon />;
    if (id === 'wrangler') return <WranglerIcon />;
    return <TerminalIcon />;
  };

  return (
    <div className="win" id="start">
      <aside className="side">
        <div className="lights">
          <i />
          <i />
          <i />
        </div>

        <div className="proj">
          <h3>Настройка</h3>
          <ul className="setnav">
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              Главный агент
            </li>
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              Исполнители
            </li>
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              Импорт чатов
            </li>
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              Аккаунты GitHub
            </li>
            <li aria-current="true">
              <span className="k now">
                <DotIcon />
              </span>
              Доступы
            </li>
            <li className="off">
              <span className="k todo">
                <CircleTodoIcon />
              </span>
              Инструкции
            </li>
            <li className="off">
              <span className="k todo">
                <CircleTodoIcon />
              </span>
              Локальные модели
            </li>
            <li className="off">
              <span className="k todo">
                <CircleTodoIcon />
              </span>
              Навыки
            </li>
          </ul>
        </div>

        <div className="limits">
          <h3>Шаг 5 из 8</h3>
          <div className="lim">
            <span>Настройка</span>
            <b>~6 мин</b>
            <span className="t">
              <i style={{ width: '56%' }} />
            </span>
          </div>
        </div>
      </aside>

      <main className="main">
        <section className="start">
          <h2>Доступы</h2>
          <p className="sub">
            Один раз показать агентам, куда и как ходить,{' '}
            <i>чтобы потом никто не спрашивал пароль посреди задачи.</i> Секреты остаются в Связке
            ключей, агенты получают только имена.
          </p>

          <div className="grp">
            <h3>
              Серверы по SSH <span>{sshHosts.length} хостов из ~/.ssh/config</span>
            </h3>
            <div className="rows">
              {sshHosts.map((host) => {
                const isOpen = Boolean(openSshRows[host.host]);
                const agents = host.agents || { claude: true, codex: true, agy: true };

                return (
                  <div
                    key={host.host}
                    className={`row ${isOpen ? 'open' : ''}`}
                    onClick={() => toggleSshRow(host.host)}
                  >
                    <span className="ic">
                      <ServerHostIcon />
                    </span>
                    <div className="nm">
                      {host.host}
                      <span>
                        {host.hostname || host.host}
                        {host.user ? ` · пользователь ${host.user}` : ''}
                      </span>
                    </div>
                    <div className="who">
                      <i className={agents.claude ? undefined : 'off'} title="Claude">
                        К
                      </i>
                      <i className={agents.codex ? undefined : 'off'} title="Codex">
                        Cx
                      </i>
                      <i className={agents.agy ? undefined : 'off'} title="Antigravity">
                        Ag
                      </i>
                    </div>
                    <div className={`ok ${host.warn ? 'warn' : ''}`}>
                      {host.status || 'подключается'}
                    </div>

                    {isOpen && (
                      <div className="detail">
                        {host.identityFile && (
                          <div>
                            <small>Ключ</small>
                            {host.identityFile}
                          </div>
                        )}
                        {host.checked && (
                          <div>
                            <small>Проверено</small>
                            {host.checked}
                          </div>
                        )}
                        {host.usedBy && (
                          <div>
                            <small>Используют</small>
                            {host.usedBy}
                          </div>
                        )}
                        {host.port && (
                          <div>
                            <small>Порт</small>
                            {host.port}
                          </div>
                        )}
                        {host.rule && (
                          <div className="rule">
                            <b>Правило:</b> {host.rule}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          <div className="grp">
            <h3>
              Инструменты в терминале <span>нужен вход в каждый</span>
            </h3>
            <div className="rows">
              {tools.map((tool) => {
                const agents = tool.agents || { claude: true, codex: true, agy: true };
                const isWarn = tool.warn || !tool.loggedIn;
                const statusText = tool.statusText || (tool.loggedIn ? 'вошли' : 'не выполнен вход');

                return (
                  <div key={tool.id} className="row">
                    <span className="ic">{getToolIcon(tool.id)}</span>
                    <div className="nm">
                      {tool.id}
                      {tool.account && <span>{tool.account}</span>}
                    </div>
                    <div className="who">
                      <i className={agents.claude ? undefined : 'off'} title="Claude">
                        К
                      </i>
                      <i className={agents.codex ? undefined : 'off'} title="Codex">
                        Cx
                      </i>
                      <i className={agents.agy ? undefined : 'off'} title="Antigravity">
                        Ag
                      </i>
                    </div>
                    <div className={`ok ${isWarn ? 'warn' : ''}`}>{statusText}</div>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="grp update-settings">
            <h3>
              Обновления <span>стабильный канал</span>
            </h3>
            <div className="update-settings__row">
              <label className="update-settings__toggle">
                <input
                  type="checkbox"
                  checked={updateChecksEnabled}
                  onChange={(event) => setUpdateChecks(event.target.checked)}
                />
                <span className="update-settings__track" aria-hidden="true">
                  <span />
                </span>
                <span>Проверять обновления при запуске</span>
              </label>
              <button
                type="button"
                className="update-settings__check"
                disabled={updateCheckStatus === 'checking'}
                onClick={() => void checkUpdate(true)}
              >
                {updateCheckStatus === 'checking' ? 'Проверяем…' : 'Проверить сейчас'}
              </button>
            </div>
            {updateCheckMessage ? (
              <p
                className={`update-settings__status ${
                  updateCheckStatus === 'error' ? 'is-error' : ''
                }`}
                role="status"
                aria-live="polite"
              >
                {updateCheckMessage}
              </p>
            ) : null}
          </div>

          <div className="foot">
            <div className="n">
              Серверы MCP ({mcpNames}{mcpExtra > 0 ? ` и ещё ${mcpExtra}` : ''}) уже подхвачены из
              конфигов Claude, Codex и agy. Поправить можно позже в настройках.
            </div>
            <div style={{ display: 'flex', gap: '10px' }}>
              <button
                type="button"
                className="cta"
                style={{
                  background: 'var(--raised)',
                  color: 'var(--fg)',
                  boxShadow: 'var(--shadow)',
                }}
                onClick={handleOpenTerminal}
              >
                <TerminalIcon size={16} />
                <span>Открыть терминал</span>
              </button>
              <button
                type="button"
                className="cta"
                onClick={handleCreateInstructions}
              >
                {instructionsCreated ? 'Создано!' : 'Дальше: инструкции'}
                <span>
                  <ArrowRightIcon />
                </span>
              </button>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
};
