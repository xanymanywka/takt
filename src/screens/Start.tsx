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

export const Start: React.FC = () => {
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
              Серверы по SSH <span>7 хостов из ~/.ssh/config</span>
            </h3>
            <div className="rows">
              {/* Row 1: nova-welcome-1 */}
              <div
                className={`row ${openSshRows['nova-welcome-1'] ? 'open' : ''}`}
                onClick={() => toggleSshRow('nova-welcome-1')}
              >
                <span className="ic">
                  <ServerHostIcon />
                </span>
                <div className="nm">
                  nova-welcome-1
                  <span>203.0.113.10 · пользователь deploy</span>
                </div>
                <div className="who">
                  <i title="Claude">К</i>
                  <i title="Codex">Cx</i>
                  <i className="off" title="Antigravity: нет">
                    Ag
                  </i>
                </div>
                <div className="ok">подключается</div>
                {openSshRows['nova-welcome-1'] && (
                  <div className="detail">
                    <div>
                      <small>Ключ</small>id_ed25519_deploy
                    </div>
                    <div>
                      <small>Проверено</small>сегодня, 13:48
                    </div>
                    <div>
                      <small>Используют</small>Finance, Belege
                    </div>
                    <div>
                      <small>Порт</small>22
                    </div>
                    <div className="rule">
                      <b>Правило:</b> Antigravity на этот хост не ходит, здесь лежат файлы клиента под
                      NDA.
                    </div>
                  </div>
                )}
              </div>

              {/* Row 2: amasite */}
              <div
                className={`row ${openSshRows['amasite'] ? 'open' : ''}`}
                onClick={() => toggleSshRow('amasite')}
              >
                <span className="ic">
                  <ServerHostIcon />
                </span>
                <div className="nm">
                  amasite
                  <span>web.example.com · B'JOLA, AMA Digital</span>
                </div>
                <div className="who">
                  <i title="Claude">К</i>
                  <i title="Codex">Cx</i>
                  <i title="Antigravity">Ag</i>
                </div>
                <div className="ok">подключается</div>
                {openSshRows['amasite'] && (
                  <div className="detail">
                    <div>
                      <small>Ключ</small>id_ed25519_amasite
                    </div>
                    <div>
                      <small>Проверено</small>вчера, 18:20
                    </div>
                    <div>
                      <small>Используют</small>B'JOLA, AMA Digital
                    </div>
                    <div>
                      <small>Порт</small>22
                    </div>
                  </div>
                )}
              </div>

              {/* Row 3: little-game */}
              <div
                className={`row ${openSshRows['little-game'] ? 'open' : ''}`}
                onClick={() => toggleSshRow('little-game')}
              >
                <span className="ic">
                  <ServerHostIcon />
                </span>
                <div className="nm">
                  little-game
                  <span>shop.example.net · портфолио, магазин</span>
                </div>
                <div className="who">
                  <i title="Claude">К</i>
                  <i title="Codex">Cx</i>
                  <i title="Antigravity">Ag</i>
                </div>
                <div className="ok warn">ключ истекает через 9 дней</div>
                {openSshRows['little-game'] && (
                  <div className="detail">
                    <div>
                      <small>Ключ</small>id_rsa_game
                    </div>
                    <div>
                      <small>Проверено</small>сегодня, 11:15
                    </div>
                    <div>
                      <small>Используют</small>Portfolio, Store
                    </div>
                    <div>
                      <small>Порт</small>22
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>

          <div className="grp">
            <h3>
              Инструменты в терминале <span>нужен вход в каждый</span>
            </h3>
            <div className="rows">
              <div className="row">
                <span className="ic">
                  <GhIcon />
                </span>
                <div className="nm">
                  gh
                  <span>xanymanywka и you-work, переключение по проекту</span>
                </div>
                <div className="who">
                  <i title="Claude">К</i>
                  <i title="Codex">Cx</i>
                  <i title="Antigravity">Ag</i>
                </div>
                <div className="ok">вошли</div>
              </div>

              <div className="row">
                <span className="ic">
                  <CodeRabbitIcon />
                </span>
                <div className="nm">
                  coderabbit
                  <span>ревью перед мёржем, стоп-кран на major</span>
                </div>
                <div className="who">
                  <i title="Claude">К</i>
                  <i title="Codex">Cx</i>
                  <i title="Antigravity">Ag</i>
                </div>
                <div className="ok">вошли</div>
              </div>

              <div className="row">
                <span className="ic">
                  <WranglerIcon />
                </span>
                <div className="nm">
                  wrangler
                  <span>Cloudflare: Naga Express, Pages и D1</span>
                </div>
                <div className="who">
                  <i title="Claude">К</i>
                  <i title="Codex">Cx</i>
                  <i className="off" title="Antigravity: нет">
                    Ag
                  </i>
                </div>
                <div className="ok warn">вход истёк, откроем окно входа</div>
              </div>
            </div>
          </div>

          <div className="foot">
            <div className="n">
              Серверы MCP (context7, github, nova-crm и ещё 9) уже подхвачены из конфигов Claude,
              Codex и agy. Поправить можно позже в настройках.
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
