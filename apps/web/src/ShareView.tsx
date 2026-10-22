import { decodeSummary, formatDuration, formatMoney, t } from '@meetburner/core';
import { useLang } from './storage';

export function ShareView({ query }: { query: string }) {
  const [lang] = useLang();
  const summary = decodeSummary(query);
  const locale = lang === 'es' ? 'es-ES' : 'en-US';

  return (
    <main className="setup share">
      <header className="brand">
        <img src="/favicon.svg" alt="" width={56} height={56} />
        <div>
          <h1>{t(lang, 'appTitle')}</h1>
          <p className="tagline">{t(lang, 'tagline')}</p>
        </div>
      </header>
      {summary === null ? (
        <p className="muted">{t(lang, 'invalidShareLink')}</p>
      ) : (
        <section>
          <h2>{t(lang, 'meetingSummary')}</h2>
          <dl className="share-stats">
            <div>
              <dt>{t(lang, 'totalTime')}</dt>
              <dd>{formatDuration(summary.elapsedMs)}</dd>
            </div>
            <div>
              <dt>{t(lang, 'totalCost')}</dt>
              <dd className="share-cost">{formatMoney(summary.totalCost, summary.currency, locale)}</dd>
            </div>
            <div>
              <dt>{t(lang, 'costPerMinute')}</dt>
              <dd>{formatMoney(summary.costPerMinute, summary.currency, locale)}</dd>
            </div>
          </dl>
        </section>
      )}
      <a className="button primary" href="#/">
        {t(lang, 'tryIt')}
      </a>
    </main>
  );
}
