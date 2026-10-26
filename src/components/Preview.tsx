import DOMPurify from 'dompurify';
import { marked } from 'marked';
import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { INTL_TAGS, useLocale, useT } from '../i18n';
import {
  open_external,
  preview_diff,
  preview_file_src,
  preview_read,
  type PreviewKind,
  type PreviewSource,
} from '../lib/ipc';
import { previewKey, sourcesForJob, useStore, type PreviewDevice } from '../store';
import {
  ChevronRightIcon,
  DesktopIcon,
  DiffIcon,
  DocIcon,
  ExternalIcon,
  GlobeIcon,
  ImageIcon,
  MinusIcon,
  PanelIcon,
  PhoneIcon,
  PlusIcon,
  ReloadIcon,
  TabletIcon,
} from './Icons';

const KIND_ICONS: Record<PreviewKind, React.FC<{ size?: number }>> = {
  url: GlobeIcon,
  image: ImageIcon,
  doc: DocIcon,
  diff: DiffIcon,
};

const DEVICE_WIDTHS: Record<PreviewDevice, number | null> = {
  phone: 390,
  tablet: 820,
  desktop: null,
};

const DEVICE_HEIGHTS: Record<PreviewDevice, number | null> = {
  phone: 844,
  tablet: 1180,
  desktop: null,
};

const ZOOM_MIN = 50;
const ZOOM_MAX = 400;
const HUNK_FOLD_LINES = 20;

function basename(path: string): string {
  return path.split('/').pop() ?? path;
}

function extension(path: string): string {
  const name = basename(path).toLowerCase();
  const dot = name.lastIndexOf('.');
  return dot >= 0 ? name.slice(dot + 1) : '';
}

/** Адресная строка пускает только локальный dev-сервер. */
export function isLocalUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return (
      (url.protocol === 'http:' || url.protocol === 'https:') &&
      (url.hostname === 'localhost' || url.hostname === '127.0.0.1')
    );
  } catch {
    return false;
  }
}

/** «5 мин назад» через Intl.RelativeTimeFormat на языке интерфейса. */
function useRelativeTime(): (iso: string) => string {
  const locale = useLocale();
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  return useMemo(() => {
    const rtf = new Intl.RelativeTimeFormat(INTL_TAGS[locale], { numeric: 'auto', style: 'short' });
    return (iso: string) => {
      const seconds = Math.round((new Date(iso).getTime() - now) / 1000);
      if (!Number.isFinite(seconds)) return '';
      const abs = Math.abs(seconds);
      if (abs < 60) return rtf.format(0, 'minute');
      if (abs < 3600) return rtf.format(Math.round(seconds / 60), 'minute');
      if (abs < 86400) return rtf.format(Math.round(seconds / 3600), 'hour');
      return rtf.format(Math.round(seconds / 86400), 'day');
    };
  }, [locale, now]);
}

// ---------- веб ----------
const WebView: React.FC<{ source: PreviewSource }> = ({ source }) => {
  const t = useT();
  const device = useStore((state) => state.previewDevice);
  const setDevice = useStore((state) => state.setPreviewDevice);
  const [url, setUrl] = useState(source.url ?? '');
  const [draft, setDraft] = useState(source.url ?? '');
  const [invalid, setInvalid] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const stageRef = useRef<HTMLDivElement>(null);
  const [stage, setStage] = useState({ width: 0, height: 0 });

  useEffect(() => {
    setUrl(source.url ?? '');
    setDraft(source.url ?? '');
    setInvalid(false);
  }, [source.url]);

  useLayoutEffect(() => {
    const el = stageRef.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) => {
      setStage({ width: entry.contentRect.width, height: entry.contentRect.height });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const value = draft.trim();
    if (!isLocalUrl(value)) {
      setInvalid(true);
      return;
    }
    setInvalid(false);
    setUrl(value);
    setReloadKey((k) => k + 1);
  };

  const frameWidth = DEVICE_WIDTHS[device];
  const frameHeight = DEVICE_HEIGHTS[device];
  let frameStyle: React.CSSProperties = { width: '100%', height: '100%' };
  let boxStyle: React.CSSProperties = { width: '100%', height: '100%' };
  if (frameWidth && frameHeight && stage.width > 0) {
    const scale = Math.min(1, stage.width / frameWidth, stage.height / frameHeight);
    frameStyle = { width: frameWidth, height: frameHeight, transform: `scale(${scale})` };
    boxStyle = { width: frameWidth * scale, height: frameHeight * scale };
  }

  const devices: PreviewDevice[] = ['phone', 'tablet', 'desktop'];
  const deviceIcons = { phone: PhoneIcon, tablet: TabletIcon, desktop: DesktopIcon };

  return (
    <div className="pv-web">
      <form className="pv-bar" onSubmit={submit}>
        <button
          type="button"
          className="pv-icon-btn"
          onClick={() => setReloadKey((k) => k + 1)}
          title={t('preview.reload')}
          aria-label={t('preview.reload')}
        >
          <ReloadIcon size={15} />
        </button>
        <input
          className={invalid ? 'pv-address invalid' : 'pv-address'}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          aria-label={t('preview.address')}
          aria-invalid={invalid}
          spellCheck={false}
        />
        <div className="pv-seg" role="radiogroup" aria-label={t('preview.device')}>
          {devices.map((d) => {
            const Icon = deviceIcons[d];
            return (
              <button
                key={d}
                type="button"
                role="radio"
                aria-checked={device === d}
                onClick={() => setDevice(d)}
                title={t(`preview.device.${d}`)}
                aria-label={t(`preview.device.${d}`)}
              >
                <Icon size={15} />
              </button>
            );
          })}
        </div>
      </form>
      {invalid && <p className="pv-hint">{t('preview.addressInvalid')}</p>}
      <div className={`pv-stage device-${device}`} ref={stageRef}>
        <div className="pv-frame-box" style={boxStyle}>
          <iframe
            key={reloadKey}
            className="pv-frame"
            style={frameStyle}
            src={url}
            title={source.title}
            data-device={device}
          />
        </div>
      </div>
    </div>
  );
};

// ---------- картинка ----------
const ImageView: React.FC<{ source: PreviewSource; sources: PreviewSource[] }> = ({ source, sources }) => {
  const t = useT();
  const [zoom, setZoom] = useState(100);
  const [compare, setCompare] = useState(false);
  const [split, setSplit] = useState(50);
  const viewRef = useRef<HTMLDivElement>(null);

  const others = useMemo(
    () => sources.filter((s) => s.kind === 'image' && s.path && s.path !== source.path),
    [sources, source.path]
  );
  const twin = others.find((s) => basename(s.path!) === basename(source.path ?? ''));
  const [beforePath, setBeforePath] = useState<string>('');

  useEffect(() => {
    setZoom(100);
    setCompare(false);
    setBeforePath(twin?.path ?? others[0]?.path ?? '');
    // Сброс только при смене картинки; twin/others выводятся из того же source.
  }, [source.path]);

  const clampZoom = (z: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(z)));

  useEffect(() => {
    const el = viewRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      setZoom((z) => clampZoom(z * (e.deltaY < 0 ? 1.1 : 1 / 1.1)));
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  }, []);

  const src = preview_file_src(source.path ?? '');
  // Выбор мог устареть: источники приходят событием preview://changed
  const effectiveBefore = others.some((s) => s.path === beforePath)
    ? beforePath
    : (twin?.path ?? others[0]?.path ?? '');
  const beforeSrc = effectiveBefore ? preview_file_src(effectiveBefore) : '';
  const imgStyle: React.CSSProperties = { width: `${zoom}%` };

  return (
    <div className="pv-image">
      <div className="pv-bar">
        <button
          type="button"
          className="pv-icon-btn"
          onClick={() => setZoom((z) => clampZoom(z - 25))}
          disabled={zoom <= ZOOM_MIN}
          aria-label={t('preview.zoomOut')}
          title={t('preview.zoomOut')}
        >
          <MinusIcon size={15} />
        </button>
        <button type="button" className="pv-zoom" onClick={() => setZoom(100)} title={t('preview.zoomReset')}>
          {zoom}%
        </button>
        <button
          type="button"
          className="pv-icon-btn"
          onClick={() => setZoom((z) => clampZoom(z + 25))}
          disabled={zoom >= ZOOM_MAX}
          aria-label={t('preview.zoomIn')}
          title={t('preview.zoomIn')}
        >
          <PlusIcon size={15} />
        </button>
        <span className="pv-spacer" />
        {others.length > 0 && (
          <>
            {compare && (
              <select
                className="pv-select"
                value={effectiveBefore}
                onChange={(e) => setBeforePath(e.target.value)}
                aria-label={t('preview.compareWith')}
              >
                {others.map((s) => (
                  <option key={s.path} value={s.path}>
                    {s.title}
                  </option>
                ))}
              </select>
            )}
            <button
              type="button"
              className="pv-text-btn"
              aria-pressed={compare}
              onClick={() => setCompare((c) => !c)}
            >
              {t('preview.compare')}
            </button>
          </>
        )}
      </div>
      <div className="pv-image-view" ref={viewRef}>
        {compare && beforeSrc ? (
          <div className="pv-compare" style={imgStyle}>
            <img src={beforeSrc} alt={t('preview.before')} draggable={false} />
            <img
              className="pv-compare-after"
              src={src}
              alt={t('preview.after')}
              draggable={false}
              style={{ clipPath: `inset(0 0 0 ${split}%)` }}
            />
            <span className="pv-compare-line" style={{ left: `${split}%` }} />
            <span className="pv-compare-tag before">{t('preview.before')}</span>
            <span className="pv-compare-tag after">{t('preview.after')}</span>
          </div>
        ) : (
          <img src={src} alt={source.title} style={imgStyle} draggable={false} />
        )}
      </div>
      {compare && beforeSrc && (
        <input
          className="pv-split"
          type="range"
          min={0}
          max={100}
          value={split}
          onChange={(e) => setSplit(Number(e.target.value))}
          aria-label={t('preview.split')}
        />
      )}
    </div>
  );
};

// ---------- документ ----------
const DocView: React.FC<{ source: PreviewSource }> = ({ source }) => {
  const t = useT();
  const path = source.path ?? '';
  const ext = extension(path);
  const [text, setText] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (ext === 'pdf') return;
    let cancelled = false;
    setText(null);
    setError(null);
    preview_read(path)
      .then((value) => !cancelled && setText(value))
      .catch((err: unknown) => !cancelled && setError(err instanceof Error ? err.message : String(err)));
    return () => {
      cancelled = true;
    };
  }, [path, ext]);

  const markdownHtml = useMemo(() => {
    if (text === null || (ext !== 'md' && ext !== 'markdown')) return '';
    return DOMPurify.sanitize(marked.parse(text, { async: false }));
  }, [text, ext]);

  if (ext === 'pdf') {
    return <iframe className="pv-doc-frame" src={preview_file_src(path)} title={source.title} />;
  }
  if (error) return <p className="pv-hint pv-pad">{t('preview.readFailed', { message: error })}</p>;
  if (text === null) return <p className="pv-hint pv-pad">{t('preview.loading')}</p>;

  if (ext === 'html' || ext === 'htm') {
    return <iframe className="pv-doc-frame" sandbox="" srcDoc={text} title={source.title} />;
  }
  if (ext === 'svg') {
    const doc = `<!doctype html><html><body style="margin:0;display:grid;place-items:center;min-height:100vh;background:#fff">${text}</body></html>`;
    return <iframe className="pv-doc-frame" sandbox="" srcDoc={doc} title={source.title} />;
  }
  if (markdownHtml) {
    // Ссылки не должны уводить окно приложения: внешние — в браузер, остальные гасим
    const onLinkClick = (e: React.MouseEvent<HTMLElement>) => {
      const a = (e.target as HTMLElement).closest('a');
      const href = a?.getAttribute('href');
      if (!href || href.startsWith('#')) return;
      e.preventDefault();
      if (/^(https?:|mailto:)/i.test(href)) void open_external(href, true);
    };
    return (
      <article className="pv-md" onClick={onLinkClick} dangerouslySetInnerHTML={{ __html: markdownHtml }} />
    );
  }
  return <pre className="pv-plain">{text}</pre>;
};

// ---------- дифф ----------
type DiffLine = { type: 'add' | 'del' | 'ctx' | 'meta'; text: string; oldNo?: number; newNo?: number };
type DiffHunk = { header: string; lines: DiffLine[] };

export function parseUnifiedDiff(diff: string): { meta: string[]; hunks: DiffHunk[] } {
  const meta: string[] = [];
  const hunks: DiffHunk[] = [];
  let current: DiffHunk | null = null;
  let oldNo = 0;
  let newNo = 0;
  for (const raw of diff.replace(/\n$/, '').split('\n')) {
    const header = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(raw);
    if (header) {
      oldNo = Number(header[1]);
      newNo = Number(header[2]);
      current = { header: raw, lines: [] };
      hunks.push(current);
      continue;
    }
    if (!current) {
      meta.push(raw);
      continue;
    }
    if (raw.startsWith('+')) current.lines.push({ type: 'add', text: raw.slice(1), newNo: newNo++ });
    else if (raw.startsWith('-')) current.lines.push({ type: 'del', text: raw.slice(1), oldNo: oldNo++ });
    else if (raw.startsWith('\\')) current.lines.push({ type: 'meta', text: raw });
    else current.lines.push({ type: 'ctx', text: raw.slice(1), oldNo: oldNo++, newNo: newNo++ });
  }
  return { meta, hunks };
}

const Hunk: React.FC<{ hunk: DiffHunk }> = ({ hunk }) => {
  const t = useT();
  const big = hunk.lines.length > HUNK_FOLD_LINES;
  const [open, setOpen] = useState(!big);
  return (
    <div className="pv-hunk">
      <button type="button" className="pv-hunk-head" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        <ChevronRightIcon size={13} />
        <span>{hunk.header}</span>
      </button>
      {open ? (
        <table className="pv-diff-table">
          <tbody>
            {hunk.lines.map((line, idx) => (
              <tr key={idx} className={`dl ${line.type}`}>
                <td className="no">{line.oldNo ?? ''}</td>
                <td className="no">{line.newNo ?? ''}</td>
                <td className="sign">{line.type === 'add' ? '+' : line.type === 'del' ? '−' : ''}</td>
                <td className="code">{line.text}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <button type="button" className="pv-hunk-fold" onClick={() => setOpen(true)}>
          {t('preview.showLines', { count: hunk.lines.length })}
        </button>
      )}
    </div>
  );
};

const DiffView: React.FC<{ source: PreviewSource; worktree: string }> = ({ source, worktree }) => {
  const t = useT();
  const [diff, setDiff] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setDiff(null);
    setError(null);
    preview_diff(source.path ?? '', worktree)
      .then((value) => !cancelled && setDiff(value))
      .catch((err: unknown) => !cancelled && setError(err instanceof Error ? err.message : String(err)));
    return () => {
      cancelled = true;
    };
  }, [source.path, worktree]);

  const parsed = useMemo(() => (diff === null ? null : parseUnifiedDiff(diff)), [diff]);

  if (error) return <p className="pv-hint pv-pad">{t('preview.readFailed', { message: error })}</p>;
  if (!parsed) return <p className="pv-hint pv-pad">{t('preview.loading')}</p>;
  if (parsed.hunks.length === 0) return <p className="pv-hint pv-pad">{t('preview.noChanges')}</p>;

  return (
    <div className="pv-diff">
      {parsed.meta.length > 0 && <pre className="pv-diff-meta">{parsed.meta.join('\n')}</pre>}
      {parsed.hunks.map((hunk, idx) => (
        <Hunk key={`${source.path}-${idx}`} hunk={hunk} />
      ))}
    </div>
  );
};

// ---------- панель ----------
export const Preview: React.FC = () => {
  const t = useT();
  const relTime = useRelativeTime();
  const open = useStore((state) => state.previewOpen);
  const width = useStore((state) => state.previewWidth);
  const sources = useStore((state) => state.previewSources);
  const active = useStore((state) => state.previewActive);
  const newCount = useStore((state) => state.previewNew);
  const jobs = useStore((state) => state.jobs);
  const projectPath = useStore((state) => state.activeProject?.path ?? '');
  const togglePreview = useStore((state) => state.togglePreview);
  const setPreviewWidth = useStore((state) => state.setPreviewWidth);
  const openPreview = useStore((state) => state.openPreview);
  const [dragging, setDragging] = useState(false);
  const drag = useRef<{ x: number; width: number } | null>(null);

  // ⌘\ (Ctrl+\ вне macOS) — открыть/свернуть панель
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && (e.code === 'Backslash' || e.key === '\\')) {
        e.preventDefault();
        togglePreview();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [togglePreview]);

  // Окно сузилось — ширина снова в пределах 320…60 %
  useEffect(() => {
    const onResize = () => setPreviewWidth(useStore.getState().previewWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, [setPreviewWidth]);

  const onPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      e.currentTarget.setPointerCapture(e.pointerId);
      drag.current = { x: e.clientX, width };
      setDragging(true);
    },
    [width]
  );
  const onPointerMove = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!drag.current) return;
      setPreviewWidth(drag.current.width + (drag.current.x - e.clientX));
    },
    [setPreviewWidth]
  );
  const onPointerUp = useCallback(() => {
    drag.current = null;
    setDragging(false);
  }, []);
  const onResizeKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowLeft') setPreviewWidth(width + 24);
    else if (e.key === 'ArrowRight') setPreviewWidth(width - 24);
  };

  if (!open) {
    return (
      <button
        type="button"
        className="pv-rail"
        onClick={togglePreview}
        aria-label={t('preview.open')}
        title={`${t('preview.open')} · ⌘\\`}
      >
        <PanelIcon size={16} />
        <span className="pv-rail-label">{t('preview.title')}</span>
        {newCount > 0 && (
          <span className="pv-badge" aria-label={t('preview.newSources', { count: newCount })}>
            {newCount}
          </span>
        )}
      </button>
    );
  }

  const activeKey = active ? previewKey(active) : null;
  const worktree =
    (active && jobs.find((j) => sourcesForJob(j, [active]).length > 0)?.worktree) || projectPath;

  const openOutside = () => {
    if (!active) return;
    if (active.url) void open_external(active.url, true);
    else if (active.path) void open_external(active.path, false);
  };

  let body: React.ReactNode;
  if (!active) {
    body = (
      <div className="pv-empty">
        <PanelIcon size={22} />
        <p>{t('preview.empty')}</p>
      </div>
    );
  } else if (active.kind === 'url') {
    body = <WebView source={active} />;
  } else if (active.kind === 'image') {
    body = <ImageView source={active} sources={sources} />;
  } else if (active.kind === 'doc') {
    body = <DocView source={active} />;
  } else {
    body = <DiffView source={active} worktree={worktree} />;
  }

  return (
    <aside
      className={dragging ? 'preview dragging' : 'preview'}
      style={{ width }}
      aria-label={t('preview.title')}
    >
      <div
        className="pv-resize"
        role="separator"
        aria-orientation="vertical"
        aria-label={t('preview.resize')}
        aria-valuenow={width}
        tabIndex={0}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onKeyDown={onResizeKey}
      />
      <header className="pv-head">
        <div className="pv-title">
          <h4>{t('preview.title')}</h4>
          <span className="pv-count">{t('preview.sources', { count: sources.length })}</span>
          <span className="pv-spacer" />
          <button
            type="button"
            className="pv-icon-btn"
            onClick={openOutside}
            disabled={!active}
            title={t('preview.openOutside')}
            aria-label={t('preview.openOutside')}
          >
            <ExternalIcon size={15} />
          </button>
          <button
            type="button"
            className="pv-icon-btn"
            onClick={togglePreview}
            title={`${t('preview.collapse')} · ⌘\\`}
            aria-label={t('preview.collapse')}
          >
            <ChevronRightIcon size={15} />
          </button>
        </div>
        {sources.length > 0 && (
          <div className="pv-chips" role="tablist" aria-label={t('preview.sourcesLabel')}>
            {sources.map((source) => {
              const key = previewKey(source);
              const Icon = KIND_ICONS[source.kind];
              return (
                <button
                  key={key}
                  type="button"
                  role="tab"
                  aria-selected={key === activeKey}
                  className="pv-chip"
                  onClick={() => openPreview(source)}
                  title={source.path ?? source.url}
                >
                  <Icon size={13} />
                  <span className="pv-chip-title">{source.title}</span>
                  <time dateTime={source.mtime}>{relTime(source.mtime)}</time>
                </button>
              );
            })}
          </div>
        )}
      </header>
      <div className={`pv-body kind-${active?.kind ?? 'empty'}`}>{body}</div>
    </aside>
  );
};
