const SYMBOL_AFTER = new Set(['EUR']);

/** Separators follow the UI locale; the symbol position follows the currency ($1.00, 1.00 €). */
export function formatMoney(value: number, currency: string, locale: string): string {
  try {
    const parts = new Intl.NumberFormat(locale, {
      style: 'currency',
      currency,
      currencyDisplay: 'narrowSymbol',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).formatToParts(Math.abs(value));
    const symbol = parts.find((p) => p.type === 'currency')?.value ?? currency;
    const amount = parts
      .filter((p) => p.type !== 'currency' && p.type !== 'literal')
      .map((p) => p.value)
      .join('');
    const sign = value < 0 ? '-' : '';
    if (SYMBOL_AFTER.has(currency)) return `${sign}${amount} ${symbol}`;
    return `${sign}${symbol}${/[a-z]$/i.test(symbol) ? ' ' : ''}${amount}`;
  } catch {
    return `${value.toFixed(2)} ${currency}`;
  }
}

const SHORT_SCALE = ['', 'K', 'M', 'B', 'T'];
// Long scale (Spain, most of Europe): 10^9 is "mil millones" and a billion (billón) is 10^12.
const LONG_SCALE = ['', 'K', 'M', ' mil M', 'B'];

/** Money abbreviated by thousands ($1.23M, 2,50 mil M €), using the short or long scale of the locale. */
export function formatMoneyCompact(value: number, currency: string, locale: string): string {
  const suffixes = locale.startsWith('en') ? SHORT_SCALE : LONG_SCALE;
  let scaled = value;
  let unit = 0;
  while (unit < suffixes.length - 1 && Math.round(Math.abs(scaled) * 100) / 100 >= 1000) {
    scaled /= 1000;
    unit++;
  }
  return formatMoney(scaled, currency, locale).replace(/(\d)(?!.*\d)/, `$1${suffixes[unit]}`);
}

export function formatDuration(ms: number): string {
  const total = Math.floor(Math.max(0, ms) / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return [h, m, s].map((n) => String(n).padStart(2, '0')).join(':');
}

const pad = (n: number) => String(n).padStart(2, '0');

/** Epoch ms -> value for <input type="datetime-local"> in local time. */
export function toLocalInputValue(epochMs: number): string {
  const d = new Date(epochMs);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** <input type="datetime-local"> value -> epoch ms, or null if empty/invalid. */
export function fromLocalInputValue(value: string): number | null {
  if (!value) return null;
  const ms = new Date(value).getTime();
  return Number.isNaN(ms) ? null : ms;
}
