import React, { useState } from 'react';
import {
  ArrowRightIcon,
  CodeRabbitIcon,
  GhIcon,
  ServerHostIcon,
  TerminalIcon,
  WranglerIcon,
} from '../components/Icons';
import { SetupSide } from '../components/SetupSide';
import { useT } from '../i18n';
import { open_in_terminal, write_instructions } from '../lib/ipc';
import { useStore } from '../store';

export const Start: React.FC = () => {
  const t = useT();
  const setup = useStore((state) => state.setup);
  const refreshSetup = useStore((state) => state.refreshSetup);

  const [openSshRows, setOpenSshRows] = useState<Record<string, boolean>>({});

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
      void refreshSetup();
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
      <SetupSide current="access" />

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
                  <button
                    key={host.host}
                    type="button"
                    className={`row ${isOpen ? 'open' : ''}`}
                    aria-expanded={isOpen}
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
                  </button>
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
                {instructionsCreated || setup?.instructionsExists
                  ? t('start.foot.created')
                  : t('start.foot.next')}
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
