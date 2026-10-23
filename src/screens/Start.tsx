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
import { LOCALES, useLocale, useT, type Locale } from '../i18n';
import { open_in_terminal, write_instructions } from '../lib/ipc';
import { useStore } from '../store';

export const Start: React.FC = () => {
  const t = useT();
  const locale = useLocale();
  const setLocale = useStore((state) => state.setLocale);
  const setScreen = useStore((state) => state.setScreen);
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
  const mcpNames = mcpList.slice(0, 3).map((m) => m.name).join(', ') || 'context7, github, postgres';
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
          <h3>{t('start.nav.title')}</h3>
          <ul className="setnav">
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              {t('start.nav.conductor')}
            </li>
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              {t('start.nav.workers')}
            </li>
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              {t('start.nav.import')}
            </li>
            <li>
              <span className="k">
                <CheckIcon />
              </span>
              {t('start.nav.github')}
            </li>
            <li aria-current="true">
              <span className="k now">
                <DotIcon />
              </span>
              {t('start.nav.access')}
            </li>
            <li className="off">
              <span className="k todo">
                <CircleTodoIcon />
              </span>
              {t('start.nav.instructions')}
            </li>
            <li className="off">
              <span className="k todo">
                <CircleTodoIcon />
              </span>
              {t('start.nav.models')}
            </li>
            <li className="off">
              <span className="k todo">
                <CircleTodoIcon />
              </span>
              {t('start.nav.skills')}
            </li>
            <li className="about-nav" onClick={() => setScreen('about')}>
              <span className="k">
                <ArrowRightIcon />
              </span>
              {t('start.nav.about')}
            </li>
          </ul>
        </div>

        <div className="limits">
          <h3>{t('start.step', { step: 5, total: 8 })}</h3>
          <div className="lim">
            <span>{t('start.setup')}</span>
            <b>{t('start.eta', { minutes: 6 })}</b>
            <span className="t">
              <i style={{ width: '56%' }} />
            </span>
          </div>
        </div>
      </aside>

      <main className="main">
        <section className="start">
          <h2>{t('start.access.title')}</h2>
          <p className="sub">
            {t('start.access.sub1')} <i>{t('start.access.sub2')}</i> {t('start.access.sub3')}
          </p>

          <div className="grp">
            <h3>
              {t('start.ssh.title')} <span>{t('start.ssh.count', { count: sshHosts.length })}</span>
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
                        {host.user ? ` · ${t('start.ssh.user', { user: host.user })}` : ''}
                      </span>
                    </div>
                    <div className="who">
                      <i className={agents.claude ? undefined : 'off'} title="Claude">
                        {t('agent.claudeInitial')}
                      </i>
                      <i className={agents.codex ? undefined : 'off'} title="Codex">
                        Cx
                      </i>
                      <i className={agents.agy ? undefined : 'off'} title="Antigravity">
                        Ag
                      </i>
                    </div>
                    <div className={`ok ${host.warn ? 'warn' : ''}`}>
                      {host.status || t('start.ssh.connecting')}
                    </div>

                    {isOpen && (
                      <div className="detail">
                        {host.identityFile && (
                          <div>
                            <small>{t('start.ssh.key')}</small>
                            {host.identityFile}
                          </div>
                        )}
                        {host.checked && (
                          <div>
                            <small>{t('start.ssh.checked')}</small>
                            {host.checked}
                          </div>
                        )}
                        {host.usedBy && (
                          <div>
                            <small>{t('start.ssh.usedBy')}</small>
                            {host.usedBy}
                          </div>
                        )}
                        {host.port && (
                          <div>
                            <small>{t('start.ssh.port')}</small>
                            {host.port}
                          </div>
                        )}
                        {host.rule && (
                          <div className="rule">
                            <b>{t('start.ssh.rule')}:</b> {host.rule}
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
              {t('start.tools.title')} <span>{t('start.tools.sub')}</span>
            </h3>
            <div className="rows">
              {tools.map((tool) => {
                const agents = tool.agents || { claude: true, codex: true, agy: true };
                const isWarn = tool.warn || !tool.loggedIn;
                const statusText = tool.statusText || (tool.loggedIn ? t('start.tools.loggedIn') : t('start.tools.loggedOut'));

                return (
                  <div key={tool.id} className="row">
                    <span className="ic">{getToolIcon(tool.id)}</span>
                    <div className="nm">
                      {tool.id}
                      {tool.account && <span>{tool.account}</span>}
                    </div>
                    <div className="who">
                      <i className={agents.claude ? undefined : 'off'} title="Claude">
                        {t('agent.claudeInitial')}
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
              {t('start.updates.title')} <span>{t('start.updates.channel')}</span>
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
                <span>{t('start.updates.onLaunch')}</span>
              </label>
              <button
                type="button"
                className="update-settings__check"
                disabled={updateCheckStatus === 'checking'}
                onClick={() => void checkUpdate(true)}
              >
                {updateCheckStatus === 'checking' ? t('start.updates.checking') : t('start.updates.checkNow')}
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

          <div className="grp language-settings">
            <h3>
              {t('start.language.title')} <span>{t('start.language.sub')}</span>
            </h3>
            <div className="lang-row" role="radiogroup" aria-label={t('start.language.title')}>
              {LOCALES.map((code: Locale) => (
                <button
                  key={code}
                  type="button"
                  role="radio"
                  aria-checked={locale === code}
                  className={`lang-btn ${locale === code ? 'is-active' : ''}`}
                  onClick={() => setLocale(code)}
                >
                  {t(`locale.${code}`)}
                </button>
              ))}
            </div>
          </div>

          <div className="foot">
            <div className="n">
              {t('start.foot.mcp', {
                names: mcpExtra > 0 ? `${mcpNames} ${t('start.foot.more', { count: mcpExtra })}` : mcpNames,
              })}
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
                <span>{t('start.foot.openTerminal')}</span>
              </button>
              <button
                type="button"
                className="cta"
                onClick={handleCreateInstructions}
              >
                {instructionsCreated ? t('start.foot.created') : t('start.foot.next')}
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
