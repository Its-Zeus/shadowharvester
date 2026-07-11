const express = require('express');
const axios = require('axios');
const crypto = require('crypto');
const notifier = require('../../lib/notifier');

const PANEL_KEY = 'b1g';
const PANEL_NAME = 'b1g (panel.b1g.me)';
const router = express.Router();

// ─── CONFIG ─────────────────────────────────────────────────────────────────

const CONFIG = {
  BASE_URL: process.env.B1G_BASE_URL || 'https://panel.b1g.me',
  CSRF_TOKEN: process.env.B1G_CSRF_TOKEN,
  RESELLER_ID: parseInt(process.env.B1G_RESELLER_ID || '8169'),
  KEEPALIVE_INTERVAL: parseInt(process.env.B1G_KEEPALIVE_INTERVAL || '60000'),
  MAX_RETRIES: parseInt(process.env.B1G_MAX_RETRIES || '3'),
  RETRY_DELAY: parseInt(process.env.B1G_RETRY_DELAY || '2000'),
  FETCH_TIMEOUT: parseInt(process.env.B1G_FETCH_TIMEOUT || '30000'),
};

// ─── TEMPLATES ───────────────────────────────────────────────────────────────
//
// Each template defines a curated bouquet list.
// template_id is passed in the request body to select bouquets.
// Template 0 (or omitted) = ALL bouquets (legacy default behaviour).
//
// Bouquet reference (UPDATED 2026-07-11):
//   4    CINEMANIA VIP
//   1    WORLD SPORTS VIP
//   2    UK VIP
//   3    PAKISTAN VIP
//   5    INDIA VIP
//   6    INDIA REGIONAL
//   2038 SRILANKA
//   7    USA & CA VIP
//   8    GERMANY VIP
//   9    AFGHAN & IRAN VIP
//   10   BANGLA VIP
//   12   ARABIC VIP
//   13   PORTUGUESE VIP
//   14   SCANDANAVIAN VIP
//   15   HUNGRY & LITHUANIA VIP
//   21   PERU VIP
//   16   ROMANIA VIP
//   17   POLISH VIP
//   18   NETHERLANDS VIP
//   19   SPANISH VIP
//   20   TURKISH VIP
//   22   RUSSIA & UKRAINE VIP
//   23   AFRICA VIP
//   24   FRANCE VIP
//   2052 ITALY
//   2072 ALBANIA VIP
//   25   IRAN VIP
//   26   GREECE VIP
//   7092 INT | CHINA
//   6295 SK:Chinese Sport - Live
//   6269 SK:Vietnam - Live
//   7082 INT | Thailand
//   6239 SK:Malaysia Movie - Live
//   6229 SK:Indonesian - Live
//   6230 SK:Indonesian News - Live
//   6231 SK:Indonesian Entertainment - Live
//   6232 SK:Indonesian Movie - Live
//   6234 SK:Indonesian General - Live
//   6255 SK:Philippine Entertainment - Live
//   6283 SK:Philippine News - Live
//   27   BELGIUM & BULGARIA VIP
//   28   MULTICHOICE DSTV AFRICA
//   30   AUSTRALIA & CARRIBEAN VIP
//   324  VOD&SERIES
//   29   Pakistani Dramas
//   7107 vod

const ALL_BOUQUETS = [
  4, 1, 2, 3, 5, 6, 2038, 7, 8, 9, 10, 12, 13, 14, 15, 21, 16, 17, 18, 19,
  20, 22, 23, 24, 2052, 2072, 25, 26, 7092, 6295, 6269, 7082, 6239, 6229,
  6230, 6231, 6232, 6234, 6255, 6283, 27, 28, 30, 324, 29, 7107
];

const TEMPLATES = {
  // 0 = full / everything (default when no template_id is provided)
  0: {
    name: 'FULL - All Bouquets',
    bouquets: ALL_BOUQUETS
  },

  // ── English-speaking world ──────────────────────────────────────────────
  1: {
    name: 'UK',
    bouquets: [7107, 2, 1, 4, 324]
  },
  2: {
    name: 'USA & Canada',
    bouquets: [7107, 7, 1, 4, 324]
  },
  3: {
    name: 'UK + USA & Canada',
    bouquets: [7107, 2, 7, 1, 4, 324]
  },
  4: {
    name: 'Australia & Caribbean',
    bouquets: [7107, 30, 1, 4, 324]
  },
  5: {
    name: 'English Full (UK + USA + AU)',
    bouquets: [7107, 2, 7, 30, 1, 4, 324]
  },

  // ── South Asia ──────────────────────────────────────────────────────────
  6: {
    name: 'India',
    bouquets: [7107, 5, 6, 1, 4, 324]
  },
  7: {
    name: 'India + Sri Lanka',
    bouquets: [7107, 5, 6, 2038, 1, 4, 324]
  },
  8: {
    name: 'Pakistan',
    bouquets: [7107, 3, 29, 1, 4, 324]
  },
  9: {
    name: 'Pakistan + India',
    bouquets: [7107, 3, 29, 5, 6, 1, 4, 324]
  },
  10: {
    name: 'Bangladesh',
    bouquets: [7107, 10, 1, 4, 324]
  },
  11: {
    name: 'South Asia Full (India + PK + BD + LK)',
    bouquets: [7107, 5, 6, 3, 29, 10, 2038, 1, 4, 324]
  },

  // ── Middle East ─────────────────────────────────────────────────────────
  12: {
    name: 'Arabic',
    bouquets: [7107, 12, 1, 4, 324]
  },
  13: {
    name: 'Turkish',
    bouquets: [7107, 20, 1, 4, 324]
  },
  14: {
    name: 'Iran',
    bouquets: [7107, 25, 9, 1, 4, 324]
  },
  15: {
    name: 'Afghan & Iran',
    bouquets: [7107, 9, 25, 1, 4, 324]
  },
  16: {
    name: 'Middle East Full (Arabic + Turkish + Iran)',
    bouquets: [7107, 12, 20, 25, 9, 1, 4, 324]
  },

  // ── Europe ──────────────────────────────────────────────────────────────
  17: {
    name: 'Germany',
    bouquets: [7107, 8, 1, 4, 324]
  },
  18: {
    name: 'France',
    bouquets: [7107, 24, 1, 4, 324]
  },
  19: {
    name: 'Spain & Latin America',
    bouquets: [7107, 19, 21, 1, 4, 324]
  },
  20: {
    name: 'Portugal & Brazil',
    bouquets: [7107, 13, 1, 4, 324]
  },
  21: {
    name: 'Netherlands',
    bouquets: [7107, 18, 1, 4, 324]
  },
  22: {
    name: 'Italy',
    bouquets: [7107, 2052, 1, 4, 324]
  },
  23: {
    name: 'Scandinavia',
    bouquets: [7107, 14, 1, 4, 324]
  },
  24: {
    name: 'Poland',
    bouquets: [7107, 17, 1, 4, 324]
  },
  25: {
    name: 'Romania',
    bouquets: [7107, 16, 1, 4, 324]
  },
  26: {
    name: 'Greece',
    bouquets: [7107, 26, 1, 4, 324]
  },
  27: {
    name: 'Albania',
    bouquets: [7107, 2072, 1, 4, 324]
  },
  28: {
    name: 'Russia & Ukraine',
    bouquets: [7107, 22, 1, 4, 324]
  },
  29: {
    name: 'Belgium & Bulgaria',
    bouquets: [7107, 27, 1, 4, 324]
  },
  30: {
    name: 'Hungary & Lithuania',
    bouquets: [7107, 15, 1, 4, 324]
  },
  31: {
    name: 'Western Europe Full (DE + FR + ES + IT + NL + PT)',
    bouquets: [7107, 8, 24, 19, 21, 2052, 18, 13, 21, 1, 4, 324]
  },
  32: {
    name: 'Eastern Europe Full (PL + RO + HU + RU + AL + BG)',
    bouquets: [7107, 17, 16, 15, 22, 2072, 27, 26, 1, 4, 324]
  },

  // ── Africa ──────────────────────────────────────────────────────────────
  33: {
    name: 'Africa',
    bouquets: [7107, 23, 28, 1, 4, 324]
  },
  34: {
    name: 'Africa + France',
    bouquets: [7107, 23, 28, 24, 1, 4, 324]
  },

  // ── Southeast Asia ──────────────────────────────────────────────────────
  35: {
    name: 'Indonesia',
    bouquets: [7107, 6229, 6230, 6231, 6232, 6234, 1, 4, 324]
  },
  36: {
    name: 'Malaysia',
    bouquets: [7107, 6239, 1, 4, 324]
  },
  37: {
    name: 'Philippines',
    bouquets: [7107, 6255, 6283, 1, 4, 324]
  },
  38: {
    name: 'Vietnam',
    bouquets: [7107, 6269, 1, 4, 324]
  },
  39: {
    name: 'Thailand',
    bouquets: [7107, 7082, 1, 4, 324]
  },
  40: {
    name: 'Southeast Asia Full (ID + MY + PH + VN + TH)',
    bouquets: [7107,
      6229, 6230, 6231, 6232, 6234,  // Indonesia (reduced)
      6239,                            // Malaysia (reduced)
      6255, 6283, 6269, 7082,         // PH/VN/TH
      1, 4, 324
    ]
  },

  // ── East Asia ───────────────────────────────────────────────────────────
  41: {
    name: 'China',
    bouquets: [7107, 7092, 6295, 1, 4, 324]
  },

  // ── Combo / Popular bundles ─────────────────────────────────────────────
  42: {
    name: 'Asia Full (South + SE + East)',
    bouquets: [7107,
      5, 6, 3, 29, 10, 2038,          // South Asia
      6229, 6230, 6231, 6232, 6234,   // Indonesia (reduced)
      6239,                            // Malaysia (reduced)
      6255, 6283, 6269, 7082,         // PH/VN/TH
      7092, 6295,                     // China
      1, 4, 324
    ]
  },
  43: {
    name: 'Sports Only',
    bouquets: [7107, 1, 4]
  },
  44: {
    name: 'VOD & Series Only',
    bouquets: [7107, 324, 4]
  }
};

function getBouquets(templateId) {
  const id = templateId ?? 0;
  const template = TEMPLATES[id];
  if (!template) {
    throw new Error(`Unknown template_id: ${id}. Valid IDs: ${Object.keys(TEMPLATES).join(', ')}`);
  }
  return template.bouquets;
}

// ─── STATE ───────────────────────────────────────────────────────────────────

const tokens = {
  xsrfToken: process.env.B1G_XSRF_TOKEN,
  managementSession: process.env.B1G_LARAVEL_SESSION,
  lastRefresh: new Date(),
  refreshCount: 0
};

const session = {
  alive: true,
  startTime: new Date(),
  lastCheck: null,
  lastRecordsCount: 0,
  errors: 0,
  consecutiveErrors: 0
};

const stats = {
  totalRequests: 0,
  successfulRequests: 0,
  failedRequests: 0,
  totalRetries: 0,
  subscriptionsCreated: 0,
  magSubscriptionsCreated: 0,
  tokenRefreshes: 0
};

const credits = {
  current: null,
  previous: null,
  lastChecked: null
};

const activeOperations = new Map();

// ─── LOGGING ─────────────────────────────────────────────────────────────────

function log(emoji, label, message, data = null) {
  const time = new Date().toISOString().replace('T', ' ').substring(0, 19);
  console.log(`${emoji} [${time}] [${label}] ${message}`);
  if (data !== null) {
    console.log('  ↳', typeof data === 'object' ? JSON.stringify(data, null, 2).replace(/\n/g, '\n  ') : data);
  }
}

// ─── HELPERS ─────────────────────────────────────────────────────────────────

function sleep(ms) {
  return new Promise(r => setTimeout(r, ms));
}

function generateUsername() {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  return Array.from({ length: 8 }, () => chars[Math.floor(Math.random() * chars.length)]).join('');
}

function formatUptime(seconds) {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return [d && `${d}d`, h && `${h}h`, m && `${m}m`, `${s}s`].filter(Boolean).join(' ');
}

function tokenAge() {
  return Math.floor((Date.now() - tokens.lastRefresh) / 1000) + 's';
}

function normalizeMac(mac) {
  if (!mac) return null;
  const clean = mac.replace(/[^a-fA-F0-9]/g, '');
  if (clean.length !== 12) return null;
  return clean.match(/.{2}/g).join(':').toUpperCase();
}

// ─── TOKEN MANAGEMENT ────────────────────────────────────────────────────────

function parseCookies(setCookieHeaders) {
  if (!setCookieHeaders) return {};
  const headers = Array.isArray(setCookieHeaders) ? setCookieHeaders : [setCookieHeaders];
  const cookies = {};
  headers.forEach(header => {
    const [pair] = header.split(';');
    const idx = pair.indexOf('=');
    if (idx > 0) cookies[pair.slice(0, idx).trim()] = pair.slice(idx + 1).trim();
  });
  return cookies;
}

function refreshTokensFromResponse(response, label) {
  const cookies = parseCookies(response.headers['set-cookie']);
  let updated = false;

  if (cookies['XSRF-TOKEN']) {
    tokens.xsrfToken = cookies['XSRF-TOKEN'];
    tokens.lastRefresh = new Date();
    tokens.refreshCount++;
    stats.tokenRefreshes++;
    updated = true;
    log('🔄', label, 'XSRF-TOKEN refreshed');
  }

  if (cookies['laravel_session']) {
    tokens.managementSession = cookies['laravel_session'];
    tokens.lastRefresh = new Date();
    tokens.refreshCount++;
    stats.tokenRefreshes++;
    updated = true;
    log('🔄', label, 'laravel_session refreshed');
  }

  return updated;
}

function getCookies() {
  return `XSRF-TOKEN=${tokens.xsrfToken}; laravel_session=${tokens.managementSession}`;
}

function getHeaders(contentType = 'application/json') {
  return {
    'x-csrf-token': CONFIG.CSRF_TOKEN,
    'Content-Type': contentType,
    'Cookie': getCookies(),
    'x-requested-with': 'XMLHttpRequest',
    'accept': 'application/json',
    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
  };
}

// ─── RETRY WRAPPER ───────────────────────────────────────────────────────────

async function withRetry(fn, label, maxRetries = CONFIG.MAX_RETRIES) {
  let lastError;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      if (attempt > 1) log('🔁', label, `Retry attempt ${attempt}/${maxRetries}`);
      const result = await fn();
      if (attempt > 1) {
        stats.totalRetries += attempt - 1;
        log('✅', label, `Succeeded on attempt ${attempt}`);
      }
      stats.successfulRequests++;
      return result;
    } catch (err) {
      lastError = err;
      stats.failedRequests++;

      if (err.response?.status >= 400 && err.response?.status < 500) {
        log('❌', label, `Client error ${err.response.status}, not retrying`, { body: err.response.data });
        throw err;
      }

      log('⚠️', label, `Attempt ${attempt} failed: ${err.message}`);
      if (attempt < maxRetries) {
        const delay = CONFIG.RETRY_DELAY * Math.pow(2, attempt - 1);
        log('⏳', label, `Waiting ${delay}ms before retry...`);
        await sleep(delay);
      }
    }
  }

  throw new Error(`[${label}] Failed after ${maxRetries} attempts: ${lastError.message}`);
}

// ─── PANEL REQUESTS — M3U ────────────────────────────────────────────────────

async function fetchSubscriptions(searchUsername = '') {
  const label = searchUsername ? `FetchSubs:${searchUsername}` : 'FetchSubs';
  stats.totalRequests++;

  log('📡', label, searchUsername
    ? `Searching panel for "${searchUsername}" (token age: ${tokenAge()})`
    : `Fetching all subscriptions (token age: ${tokenAge()})`
  );

  const COLUMNS = [
    { data: 'id', name: 'id', searchable: true, orderable: true },
    { data: 'expired', name: 'username', searchable: true, orderable: true },
    { data: 'password', name: 'password', searchable: true, orderable: true },
    { data: 'exp_date_show', name: 'users.exp_date', searchable: true, orderable: true },
    { data: 'admin_notes_show', name: 'reseller_notes', searchable: true, orderable: true },
    { data: 'speed', name: 'speed', searchable: false, orderable: false },
    { data: 'connections', name: 'active_connections', searchable: false, orderable: true },
    { data: 'display_name', name: 'streams.stream_display_name', searchable: true, orderable: true },
    { data: 'watch_ip_show', name: 'con_activities.user_ip', searchable: false, orderable: false },
    { data: 'owner', name: 'members.username', searchable: false, orderable: true },
    { data: 'vpn', name: 'users.is_restreamer', searchable: false, orderable: true },
    { data: 'created_at', name: 'created_at', searchable: false, orderable: true },
    { data: 'action', name: 'action', searchable: false, orderable: false },
  ].map(col => ({ ...col, search: { value: '', regex: false } }));

  const payload = {
    draw: 1,
    columns: COLUMNS,
    order: [{ column: 0, dir: 'desc' }],
    start: 0,
    length: searchUsername ? 10 : 500,
    search: { value: searchUsername, regex: false },
    id: 'users',
    filter: '',
    reseller: CONFIG.RESELLER_ID
  };

  const formBody = new URLSearchParams();
  const flattenToForm = (obj, prefix = '') => {
    for (const [key, val] of Object.entries(obj)) {
      const fullKey = prefix ? `${prefix}[${key}]` : key;
      if (val !== null && typeof val === 'object' && !Array.isArray(val)) {
        flattenToForm(val, fullKey);
      } else if (Array.isArray(val)) {
        val.forEach((item, i) => {
          if (typeof item === 'object') flattenToForm(item, `${fullKey}[${i}]`);
          else formBody.append(`${fullKey}[${i}]`, item);
        });
      } else {
        formBody.append(fullKey, val ?? '');
      }
    }
  };
  flattenToForm(payload);

  const response = await axios.post(`${CONFIG.BASE_URL}/lines/data`, formBody.toString(), {
    headers: {
      ...getHeaders('application/x-www-form-urlencoded; charset=UTF-8'),
      'accept': 'application/json, text/javascript, */*; q=0.01',
      'sec-fetch-dest': 'empty', 'sec-fetch-mode': 'cors', 'sec-fetch-site': 'same-origin',
      'referer': `${CONFIG.BASE_URL}/`,
    },
    timeout: CONFIG.FETCH_TIMEOUT
  });

  refreshTokensFromResponse(response, label);
  const total = response.data?.recordsTotal ?? '?';
  const returned = response.data?.data?.length ?? 0;
  log('✅', label, `Got ${returned} records (total: ${total})`);
  return response.data;
}

async function createSubscription(packageId, templateId, note = '') {
  const username = generateUsername();
  const endpoint = packageId === 56 ? 1 : 0;
  const url = `${CONFIG.BASE_URL}/lines/create/${endpoint}`;
  const label = `Create:${username}`;

  const bouquets = getBouquets(templateId);
  const bouquetsStr = bouquets.join(',');
  const formBody = `current_bouquets=${encodeURIComponent(bouquetsStr)}&_token=${encodeURIComponent(CONFIG.CSRF_TOKEN)}&line_type=line&username=${username}&password=&mac=&package=${packageId}&q=&q=&description=${note ? encodeURIComponent(note) : ''}`;

  log('🆕', label, `Creating M3U subscription`, { packageId, templateId, bouquetCount: bouquets.length, tokenAge: tokenAge() });
  stats.totalRequests++;

  const response = await axios.post(url, formBody, {
    headers: {
      ...getHeaders('application/x-www-form-urlencoded; charset=UTF-8'),
      'accept': 'application/json, text/javascript, */*; q=0.01',
      'sec-fetch-dest': 'empty', 'sec-fetch-mode': 'cors', 'sec-fetch-site': 'same-origin',
      'referer': `${CONFIG.BASE_URL}/`,
      'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36'
    },
    timeout: CONFIG.FETCH_TIMEOUT,
    validateStatus: () => true
  });

  refreshTokensFromResponse(response, label);
  log('📬', label, `Panel responded`, { status: response.status, data: response.data });

  if (response.status >= 200 && response.status < 300) {
    if (response.data?.error || response.data?.errors) {
      const msg = response.data.error || JSON.stringify(response.data.errors);
      log('❌', label, `Panel returned an error`, { error: msg });
      throw new Error(`Panel error: ${msg}`);
    }
    log('✅', label, `M3U subscription created`);
    stats.subscriptionsCreated++;
    return username;
  }

  if (response.status === 500) {
    log('⚠️', label, `Got HTTP 500 — subscription may still have been created`);
    stats.subscriptionsCreated++;
    return username;
  }

  throw new Error(`HTTP ${response.status}: ${JSON.stringify(response.data)}`);
}

async function renewSubscription(subscriptionId, packageId, note = '') {
  const url = `${CONFIG.BASE_URL}/lines/extend/${subscriptionId}`;
  const label = `Renew:${subscriptionId}`;

  const bouquetsStr = ALL_BOUQUETS.join(',');
  const formBody = `current_bouquets=${encodeURIComponent(bouquetsStr)}&_token=${encodeURIComponent(CONFIG.CSRF_TOKEN)}&package=${packageId}&q=&q=&description=${note ? encodeURIComponent(note) : ''}`;

  log('🔃', label, `Renewing subscription`, { packageId, tokenAge: tokenAge() });
  stats.totalRequests++;

  const response = await axios.post(url, formBody, {
    headers: {
      ...getHeaders('application/x-www-form-urlencoded; charset=UTF-8'),
      'accept': 'application/json, text/javascript, */*; q=0.01',
      'sec-fetch-dest': 'empty', 'sec-fetch-mode': 'cors', 'sec-fetch-site': 'same-origin',
      'referer': `${CONFIG.BASE_URL}/`,
      'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36'
    },
    timeout: CONFIG.FETCH_TIMEOUT,
    validateStatus: () => true
  });

  refreshTokensFromResponse(response, label);
  log('📬', label, `Panel responded`, { status: response.status });

  if (response.status >= 200 && response.status < 300) {
    if (response.data?.error || response.data?.errors) {
      const msg = response.data.error || JSON.stringify(response.data.errors);
      log('❌', label, `Panel returned an error`, { error: msg });
      throw new Error(`Panel error: ${msg}`);
    }
    log('✅', label, `Subscription renewed`);
    return true;
  }

  if (response.status === 500) {
    log('⚠️', label, `Got HTTP 500 — renewal may still have succeeded`);
    return true;
  }

  throw new Error(`HTTP ${response.status}: ${JSON.stringify(response.data)}`);
}

// ─── PANEL REQUESTS — MAG ────────────────────────────────────────────────────

async function fetchMagSubscriptions(searchValue = '') {
  const label = searchValue ? `FetchMag:${searchValue}` : 'FetchMag';
  stats.totalRequests++;

  log('📡', label, searchValue
    ? `Searching MAG panel for "${searchValue}" (token age: ${tokenAge()})`
    : `Fetching all MAG subscriptions (token age: ${tokenAge()})`
  );

  const COLUMNS = [
    { data: 'id', name: 'users.id', searchable: true, orderable: true },
    { data: 'expired', name: 'mag_devices.mac', searchable: true, orderable: true },
    { data: 'exp_date_show', name: 'users.exp_date', searchable: true, orderable: true },
    { data: 'stb_type', name: 'mag_devices.stb_type', searchable: true, orderable: true },
    { data: 'admin_notes_show', name: 'users.reseller_notes', searchable: true, orderable: true },
    { data: 'speed', name: 'speed', searchable: false, orderable: false },
    { data: 'connections', name: 'active_connections', searchable: false, orderable: true },
    { data: 'display_name', name: 'streams.stream_display_name', searchable: true, orderable: true },
    { data: 'watch_ip_show', name: 'con_activities.user_ip', searchable: false, orderable: false },
    { data: 'owner', name: 'members.username', searchable: false, orderable: true },
    { data: 'vpn', name: 'users.is_restreamer', searchable: false, orderable: true },
    { data: 'created_at', name: 'created_at', searchable: false, orderable: true },
    { data: 'action', name: 'action', searchable: false, orderable: false },
  ].map(col => ({ ...col, search: { value: '', regex: false } }));

  const payload = {
    draw: 1,
    columns: COLUMNS,
    order: [{ column: 0, dir: 'desc' }],
    start: 0,
    length: searchValue ? 10 : 500,
    search: { value: searchValue, regex: false },
    id: 'mags',
    filter: '',
    reseller: CONFIG.RESELLER_ID
  };

  const formBody = new URLSearchParams();
  const flattenToForm = (obj, prefix = '') => {
    for (const [key, val] of Object.entries(obj)) {
      const fullKey = prefix ? `${prefix}[${key}]` : key;
      if (val !== null && typeof val === 'object' && !Array.isArray(val)) {
        flattenToForm(val, fullKey);
      } else if (Array.isArray(val)) {
        val.forEach((item, i) => {
          if (typeof item === 'object') flattenToForm(item, `${fullKey}[${i}]`);
          else formBody.append(`${fullKey}[${i}]`, item);
        });
      } else {
        formBody.append(fullKey, val ?? '');
      }
    }
  };
  flattenToForm(payload);

  const response = await axios.post(`${CONFIG.BASE_URL}/lines/mag-data`, formBody.toString(), {
    headers: {
      ...getHeaders('application/x-www-form-urlencoded; charset=UTF-8'),
      'accept': 'application/json, text/javascript, */*; q=0.01',
      'sec-fetch-dest': 'empty', 'sec-fetch-mode': 'cors', 'sec-fetch-site': 'same-origin',
      'referer': `${CONFIG.BASE_URL}/`,
    },
    timeout: CONFIG.FETCH_TIMEOUT
  });

  refreshTokensFromResponse(response, label);
  const total = response.data?.recordsTotal ?? '?';
  const returned = response.data?.data?.length ?? 0;
  log('✅', label, `Got ${returned} MAG records (total: ${total})`);
  return response.data;
}

async function createMagSubscription(packageId, mac, templateId, note = '') {
  const normalizedMac = normalizeMac(mac);
  if (!normalizedMac) throw new Error(`Invalid MAC address: "${mac}"`);

  const url = `${CONFIG.BASE_URL}/lines/create/1`;
  const label = `CreateMAG:${normalizedMac}`;

  const bouquets = getBouquets(templateId);
  const bouquetsStr = bouquets.join(',');
  const formBody = `current_bouquets=${encodeURIComponent(bouquetsStr)}&_token=${encodeURIComponent(CONFIG.CSRF_TOKEN)}&line_type=mag&username=&mac=${encodeURIComponent(normalizedMac)}&package=${packageId}&q=&q=&description=${note ? encodeURIComponent(note) : ''}`;

  log('🆕', label, `Creating MAG subscription`, { packageId, mac: normalizedMac, templateId, bouquetCount: bouquets.length, tokenAge: tokenAge() });
  stats.totalRequests++;

  const response = await axios.post(url, formBody, {
    headers: {
      ...getHeaders('application/x-www-form-urlencoded; charset=UTF-8'),
      'accept': 'application/json, text/javascript, */*; q=0.01',
      'sec-fetch-dest': 'empty', 'sec-fetch-mode': 'cors', 'sec-fetch-site': 'same-origin',
      'referer': `${CONFIG.BASE_URL}/`,
      'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36'
    },
    timeout: CONFIG.FETCH_TIMEOUT,
    validateStatus: () => true
  });

  refreshTokensFromResponse(response, label);
  log('📬', label, `Panel responded`, { status: response.status, data: response.data });

  if (response.status >= 200 && response.status < 300) {
    if (response.data?.error || response.data?.errors) {
      const msg = response.data.error || JSON.stringify(response.data.errors);
      log('❌', label, `Panel returned an error`, { error: msg });
      throw new Error(`Panel error: ${msg}`);
    }
    log('✅', label, `MAG subscription created`);
    stats.magSubscriptionsCreated++;
    return normalizedMac;
  }

  if (response.status === 500) {
    log('⚠️', label, `Got HTTP 500 — MAG subscription may still have been created`);
    stats.magSubscriptionsCreated++;
    return normalizedMac;
  }

  throw new Error(`HTTP ${response.status}: ${JSON.stringify(response.data)}`);
}

// ─── FIND SUBSCRIPTION ───────────────────────────────────────────────────────

async function findSubscription(username, maxAttempts = 10) {
  const label = `Find:${username}`;
  log('🔍', label, `Starting search`, { maxAttempts });

  for (let i = 1; i <= maxAttempts; i++) {
    log('🔎', label, `Attempt ${i}/${maxAttempts}`);
    let data;
    try {
      data = await withRetry(() => fetchSubscriptions(username), label, 2);
    } catch (err) {
      log('❌', label, `Fetch failed on attempt ${i}`, { error: err.message });
      if (i === maxAttempts) throw err;
      await sleep(500);
      continue;
    }

    const results = data.data ?? [];
    const total = data.recordsTotal ?? 0;

    if (results.length === 0 && total > 0) {
      log('🚨', label, `Session issue — panel has ${total} records but returned 0`);
    }

    const match = results.find(s => s.username === username);
    if (match) {
      log('✅', label, `Found on attempt ${i}`, { id: match.id, exp_date: match.exp_date });
      return extractSubFields(match);
    }

    log('⏳', label, `Not found yet (panel returned ${results.length} results)`);
    if (i < maxAttempts) await sleep(500);
  }

  log('❌', label, `Gave up after ${maxAttempts} attempts`);
  throw new Error(`Subscription "${username}" not found after ${maxAttempts} attempts`);
}

async function findMagSubscription(mac, maxAttempts = 10) {
  const normalizedMac = normalizeMac(mac);
  if (!normalizedMac) throw new Error(`Invalid MAC address: "${mac}"`);

  const label = `FindMAG:${normalizedMac}`;
  log('🔍', label, `Starting MAG search`, { maxAttempts });

  for (let i = 1; i <= maxAttempts; i++) {
    log('🔎', label, `Attempt ${i}/${maxAttempts}`);
    let data;
    try {
      data = await withRetry(() => fetchMagSubscriptions(normalizedMac), label, 2);
    } catch (err) {
      log('❌', label, `Fetch failed on attempt ${i}`, { error: err.message });
      if (i === maxAttempts) throw err;
      await sleep(500);
      continue;
    }

    const results = data.data ?? [];
    const total = data.recordsTotal ?? 0;

    if (results.length === 0 && total > 0) {
      log('🚨', label, `Session issue — panel has ${total} MAG records but returned 0`);
    }

    const match = results.find(s => normalizeMac(s.mac) === normalizedMac);
    if (match) {
      log('✅', label, `Found MAG on attempt ${i}`, { id: match.id, mac: match.mac });
      return extractMagSubFields(match);
    }

    log('⏳', label, `Not found yet (panel returned ${results.length} results)`);
    if (i < maxAttempts) await sleep(500);
  }

  log('❌', label, `Gave up after ${maxAttempts} attempts`);
  throw new Error(`MAG subscription for MAC "${normalizedMac}" not found after ${maxAttempts} attempts`);
}

// ─── CREDITS FETCH ───────────────────────────────────────────────────────────

async function fetchCredits() {
  const label = 'Credits';
  stats.totalRequests++;

  const response = await axios.get(`${CONFIG.BASE_URL}/lines`, {
    headers: {
      'accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7',
      'accept-language': 'en-US,en;q=0.9,fr;q=0.8',
      'cache-control': 'max-age=0',
      'sec-ch-ua': '"Chromium";v="146", "Not-A.Brand";v="24", "Google Chrome";v="146"',
      'sec-ch-ua-mobile': '?0',
      'sec-ch-ua-platform': '"Windows"',
      'sec-fetch-dest': 'document',
      'sec-fetch-mode': 'navigate',
      'sec-fetch-site': 'same-origin',
      'sec-fetch-user': '?1',
      'upgrade-insecure-requests': '1',
      'Cookie': getCookies(),
      'referer': `${CONFIG.BASE_URL}/`,
    },
    timeout: CONFIG.FETCH_TIMEOUT
  });

  refreshTokensFromResponse(response, label);

  // Extract credits from HTML: <div class="label label-warning">Credits: 47</div>
  const html = response.data;
  const match = html.match(/Credits:\s*(\d+)/);
  const amount = match ? parseInt(match[1]) : null;

  if (amount !== null) {
    credits.previous = credits.current;
    credits.current = amount;
    credits.lastChecked = new Date();
    stats.successfulRequests++;
  } else {
    log('⚠️', label, 'Could not extract credits from HTML response');
  }

  return amount;
}

// ─── SESSION MANAGEMENT ──────────────────────────────────────────────────────

async function validateSession() {
  log('🔐', 'Session', 'Validating session...');
  try {
    const amount = await fetchCredits();
    session.alive = true;
    notifier.markAlive(PANEL_KEY, PANEL_NAME);
    log('✅', 'Session', `Session is alive`, { credits: amount, resellerId: CONFIG.RESELLER_ID });
    return true;
  } catch (err) {
    session.alive = false;
    notifier.markDead(PANEL_KEY, PANEL_NAME, err.message);
    log('❌', 'Session', `Validation failed`, { error: err.message, status: err.response?.status });
    return false;
  }
}

async function keepAlive() {
  try {
    const amount = await fetchCredits();

    session.lastCheck = new Date();
    session.consecutiveErrors = 0;

    const uptime = formatUptime(Math.floor((Date.now() - session.startTime) / 1000));
    log('💓', 'KeepAlive', `Session alive (uptime: ${uptime})`, {
      credits: amount, tokenAge: tokenAge(), tokenRefreshes: tokens.refreshCount
    });

    if (amount === null) {
      log('🚨', 'KeepAlive', `SESSION EXPIRED — could not read credits from page`);
      session.alive = false;
      notifier.markDead(PANEL_KEY, PANEL_NAME, 'Keep-alive could not read credits — session likely expired');
    } else {
      // Healthy read — (re)assert alive so the session auto-recovers after a fix.
      session.alive = true;
      notifier.markAlive(PANEL_KEY, PANEL_NAME);
    }
  } catch (err) {
    session.errors++;
    session.consecutiveErrors++;
    log('⚠️', 'KeepAlive', `Error`, { error: err.message, consecutive: session.consecutiveErrors });
    if (session.consecutiveErrors >= 5) {
      log('💀', 'KeepAlive', `Too many consecutive errors — marking session as dead`);
      session.alive = false;
      notifier.markDead(PANEL_KEY, PANEL_NAME, `Keep-alive failing: ${err.message}`);
    }
  }
}

async function startKeepAlive() {
  log('🚀', 'Startup', `Initializing session monitor`, { keepaliveInterval: CONFIG.KEEPALIVE_INTERVAL + 'ms' });
  const valid = await validateSession();
  if (!valid) log('⚠️', 'Startup', `Session validation failed — check your CSRF token and cookies`);
  setInterval(keepAlive, CONFIG.KEEPALIVE_INTERVAL);
  log('💓', 'Startup', `Keep-alive monitor started`);
}

// ─── UTILITIES ───────────────────────────────────────────────────────────────

function extractSubFields(sub) {
  return {
    id: sub.id,
    username: sub.username,
    password: sub.password,
    exp_date: sub.exp_date,
    owner: sub.owner,
    created_at: sub.created_at,
    reseller_notes: sub.reseller_notes,
    connections: sub.connections,
    is_trial: sub.is_trial,
    enabled: sub.enabled,
    active_connections: sub.active_connections
  };
}

function extractMagSubFields(sub) {
  return {
    id: sub.id,
    mac: sub.mac,
    username: sub.username,
    password: sub.password,
    stb_type: sub.stb_type,
    exp_date: sub.exp_date,
    expire_date: sub.expire_date,
    owner: sub.owner,
    created_at: sub.created_at,
    reseller_notes: sub.reseller_notes,
    connections: sub.connections,
    is_trial: sub.is_trial,
    enabled: sub.enabled,
    admin_enabled: sub.admin_enabled,
    active_connections: sub.active_connections,
    lock_device: sub.lock_device
  };
}

// ─── API ROUTES — TEMPLATES ──────────────────────────────────────────────────

/**
 * GET /templates
 * Returns all available template IDs with their names and bouquet counts.
 */
router.get('/templates', (req, res) => {
  const list = Object.entries(TEMPLATES).map(([id, t]) => ({
    template_id: parseInt(id),
    name: t.name,
    bouquet_count: t.bouquets.length,
    bouquets: t.bouquets
  }));
  res.json({ success: true, count: list.length, templates: list });
});

// ─── API ROUTES — M3U ────────────────────────────────────────────────────────

router.get('/health', (req, res) => {
  const uptime = Math.floor((Date.now() - session.startTime) / 1000);
  res.json({
    status: session.alive ? 'alive' : 'dead',
    session: {
      startTime: session.startTime, lastCheck: session.lastCheck,
      aliveForSeconds: uptime, errors: session.errors,
      consecutiveErrors: session.consecutiveErrors, lastRecordsCount: session.lastRecordsCount
    },
    tokens: {
      lastRefresh: tokens.lastRefresh,
      tokenAgeSeconds: Math.floor((Date.now() - tokens.lastRefresh) / 1000),
      refreshCount: tokens.refreshCount
    },
    stats,
    credits: {
      current: credits.current,
      previous: credits.previous,
      lastChecked: credits.lastChecked
    },
    config: {
      baseUrl: CONFIG.BASE_URL, resellerId: CONFIG.RESELLER_ID,
      keepaliveInterval: CONFIG.KEEPALIVE_INTERVAL, maxRetries: CONFIG.MAX_RETRIES
    }
  });
});

router.get('/stats', (req, res) => {
  const uptime = Math.floor((Date.now() - session.startTime) / 1000);
  res.json({
    uptime: { seconds: uptime, formatted: formatUptime(uptime) },
    session: {
      alive: session.alive, totalErrors: session.errors,
      consecutiveErrors: session.consecutiveErrors, lastCheck: session.lastCheck
    },
    tokens: {
      lastRefresh: tokens.lastRefresh,
      ageSeconds: Math.floor((Date.now() - tokens.lastRefresh) / 1000),
      refreshCount: tokens.refreshCount
    },
    requests: {
      ...stats,
      successRate: stats.totalRequests > 0
        ? ((stats.successfulRequests / stats.totalRequests) * 100).toFixed(2) + '%'
        : 'N/A'
    },
    activeOperations: activeOperations.size
  });
});

/**
 * POST /subscription/create
 * Body: { package: number, template_id?: number, note?: string }
 *
 * template_id is optional. Omit or pass 0 for all bouquets.
 */
router.post('/subscription/create', async (req, res) => {
  const { package: packageId, template_id, note } = req.body;
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-Create:${requestId}`;

  log('📥', label, `Create request received`, { packageId, template_id, note });

  if (!packageId || typeof packageId !== 'number') {
    return res.status(400).json({ success: false, error: 'package must be a number', requestId });
  }

  // Validate template_id if provided
  if (template_id !== undefined && template_id !== null) {
    if (typeof template_id !== 'number' || !TEMPLATES[template_id]) {
      return res.status(400).json({
        success: false,
        error: `Invalid template_id: ${template_id}. Valid IDs: ${Object.keys(TEMPLATES).join(', ')}`,
        requestId
      });
    }
  }

  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive. Check /health.', requestId });
  }

  const resolvedTemplateId = template_id ?? 0;
  const opKey = `pkg-${packageId}-tpl-${resolvedTemplateId}`;

  try {
    if (activeOperations.has(opKey)) {
      log('♻️', label, `Duplicate request — waiting for existing operation`);
      const result = await activeOperations.get(opKey);
      return res.json({ ...result, duplicate: true, requestId });
    }

    const operation = (async () => {
      try {
        let username;
        let creationMayHaveFailed = false;

        try {
          username = await withRetry(() => createSubscription(packageId, resolvedTemplateId, note), label);
        } catch (err) {
          if (err.message.includes('500')) {
            log('⚠️', label, `Creation returned 500 — checking if it went through anyway`);
            creationMayHaveFailed = true;
          } else {
            throw err;
          }
        }

        log('⏳', label, `Waiting 200ms before searching...`);
        await sleep(200);

        let subscription;
        if (creationMayHaveFailed) {
          const data = await fetchSubscriptions();
          const latest = data.data?.[0];
          if (!latest) throw new Error('Creation failed and no subscriptions found in panel');
          log('✅', label, `Using latest record as fallback`, { username: latest.username });
          subscription = extractSubFields(latest);
        } else {
          subscription = await findSubscription(username, 10);
        }

        log('🎉', label, `Create workflow complete`, { username: subscription.username, id: subscription.id });
        return {
          success: true,
          subscription,
          template: { id: resolvedTemplateId, name: TEMPLATES[resolvedTemplateId].name },
          requestId
        };
      } finally {
        activeOperations.delete(opKey);
      }
    })();

    activeOperations.set(opKey, operation);
    const result = await operation;
    res.json(result);

  } catch (err) {
    activeOperations.delete(opKey);
    log('❌', label, `Create failed`, { error: err.message });

    // Check if credits decreased — if so, we lost a credit without getting the sub
    const creditsBefore = credits.current;
    try {
      const creditsNow = await fetchCredits();
      log('💰', label, `Credits check after failure`, { before: creditsBefore, after: creditsNow });
      if (creditsBefore !== null && creditsNow !== null && creditsNow < creditsBefore) {
        log('🛑', label, `Credits decreased (${creditsBefore} → ${creditsNow}) — shutting down to prevent further loss`);
        res.status(500).json({ success: false, error: err.message, creditsLost: true, requestId });
        setTimeout(() => process.exit(1), 500);
        return;
      } else {
        log('✅', label, `Credits unchanged (${creditsNow}) — panel hiccup, staying alive`);
      }
    } catch (creditsErr) {
      log('⚠️', label, `Could not check credits after failure`, { error: creditsErr.message });
    }

    res.status(500).json({ success: false, error: err.message, requestId });
  }
});

router.post('/subscription/renew', async (req, res) => {
  const { id, package: packageId, note } = req.body;
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-Renew:${requestId}`;

  log('📥', label, `Renew request received`, { id, packageId });

  if (!id || typeof id !== 'number') {
    return res.status(400).json({ success: false, error: 'id must be a number', requestId });
  }
  if (!packageId || typeof packageId !== 'number') {
    return res.status(400).json({ success: false, error: 'package must be a number', requestId });
  }
  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive. Check /health.', requestId });
  }

  try {
    await withRetry(() => renewSubscription(id, packageId, note), label);
    log('🎉', label, `Renew complete`, { id, packageId });
    res.json({ success: true, message: `Subscription ${id} renewed with package ${packageId}`, subscriptionId: id, requestId });
  } catch (err) {
    log('❌', label, `Renew failed`, { error: err.message });

    const creditsBefore = credits.current;
    try {
      const creditsNow = await fetchCredits();
      log('💰', label, `Credits check after failure`, { before: creditsBefore, after: creditsNow });
      if (creditsBefore !== null && creditsNow !== null && creditsNow < creditsBefore) {
        log('🛑', label, `Credits decreased (${creditsBefore} → ${creditsNow}) — shutting down to prevent further loss`);
        res.status(500).json({ success: false, error: err.message, creditsLost: true, requestId });
        setTimeout(() => process.exit(1), 500);
        return;
      } else {
        log('✅', label, `Credits unchanged (${creditsNow}) — panel hiccup, staying alive`);
      }
    } catch (creditsErr) {
      log('⚠️', label, `Could not check credits after failure`, { error: creditsErr.message });
    }

    res.status(500).json({ success: false, error: err.message, requestId });
  }
});

router.get('/subscription/:username', async (req, res) => {
  const { username } = req.params;
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-Get:${requestId}`;

  log('📥', label, `Get request`, { username });

  if (!username || username.length !== 8) {
    return res.status(400).json({ success: false, error: 'Username must be 8 characters', requestId });
  }
  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive', requestId });
  }

  try {
    const subscription = await withRetry(() => findSubscription(username, 3), label);
    res.json({ success: true, subscription, requestId });
  } catch (err) {
    log('❌', label, `Not found`, { username, error: err.message });
    res.status(404).json({ success: false, error: err.message, requestId });
  }
});

router.get('/subscriptions', async (req, res) => {
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-List:${requestId}`;

  log('📥', label, `List request`);
  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive', requestId });
  }

  try {
    const data = await withRetry(() => fetchSubscriptions(), label);
    const subscriptions = data.data.map(extractSubFields);
    log('✅', label, `Listed ${subscriptions.length} subscriptions`);
    res.json({ success: true, total: data.recordsTotal, filtered: data.recordsFiltered, count: subscriptions.length, subscriptions, requestId });
  } catch (err) {
    log('❌', label, `List failed`, { error: err.message });
    res.status(500).json({ success: false, error: err.message, requestId });
  }
});

// ─── API ROUTES — MAG ────────────────────────────────────────────────────────

/**
 * POST /mag/create
 * Body: { package: number, mac: string, template_id?: number, note?: string }
 */
router.post('/mag/create', async (req, res) => {
  const { package: packageId, mac, template_id, note } = req.body;
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-MAGCreate:${requestId}`;

  log('📥', label, `MAG create request received`, { packageId, mac, template_id, note });

  if (!packageId || typeof packageId !== 'number') {
    return res.status(400).json({ success: false, error: 'package must be a number', requestId });
  }
  if (!mac || typeof mac !== 'string') {
    return res.status(400).json({ success: false, error: 'mac is required', requestId });
  }

  const normalizedMac = normalizeMac(mac);
  if (!normalizedMac) {
    return res.status(400).json({ success: false, error: `Invalid MAC address format: "${mac}"`, requestId });
  }

  if (template_id !== undefined && template_id !== null) {
    if (typeof template_id !== 'number' || !TEMPLATES[template_id]) {
      return res.status(400).json({
        success: false,
        error: `Invalid template_id: ${template_id}. Valid IDs: ${Object.keys(TEMPLATES).join(', ')}`,
        requestId
      });
    }
  }

  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive. Check /health.', requestId });
  }

  const resolvedTemplateId = template_id ?? 0;
  const opKey = `mag-${normalizedMac}-${packageId}-tpl-${resolvedTemplateId}`;

  try {
    if (activeOperations.has(opKey)) {
      log('♻️', label, `Duplicate MAG request — waiting for existing operation`);
      const result = await activeOperations.get(opKey);
      return res.json({ ...result, duplicate: true, requestId });
    }

    const operation = (async () => {
      try {
        let creationMayHaveFailed = false;

        try {
          await withRetry(() => createMagSubscription(packageId, normalizedMac, resolvedTemplateId, note), label);
        } catch (err) {
          if (err.message.includes('500')) {
            log('⚠️', label, `MAG creation returned 500 — checking if it went through anyway`);
            creationMayHaveFailed = true;
          } else {
            throw err;
          }
        }

        log('⏳', label, `Waiting 200ms before searching...`);
        await sleep(200);

        let subscription;
        if (creationMayHaveFailed) {
          const data = await fetchMagSubscriptions();
          const latest = data.data?.[0];
          if (!latest) throw new Error('MAG creation failed and no records found in panel');
          log('✅', label, `Using latest MAG record as fallback`, { mac: latest.mac, id: latest.id });
          subscription = extractMagSubFields(latest);
        } else {
          subscription = await findMagSubscription(normalizedMac, 10);
        }

        log('🎉', label, `MAG create workflow complete`, { mac: subscription.mac, id: subscription.id });
        return {
          success: true,
          subscription,
          template: { id: resolvedTemplateId, name: TEMPLATES[resolvedTemplateId].name },
          requestId
        };
      } finally {
        activeOperations.delete(opKey);
      }
    })();

    activeOperations.set(opKey, operation);
    const result = await operation;
    res.json(result);

  } catch (err) {
    activeOperations.delete(opKey);
    log('❌', label, `MAG create failed`, { error: err.message });

    // Check if credits decreased — if so, we lost a credit without getting the sub
    const creditsBefore = credits.current;
    try {
      const creditsNow = await fetchCredits();
      log('💰', label, `Credits check after failure`, { before: creditsBefore, after: creditsNow });
      if (creditsBefore !== null && creditsNow !== null && creditsNow < creditsBefore) {
        log('🛑', label, `Credits decreased (${creditsBefore} → ${creditsNow}) — shutting down to prevent further loss`);
        res.status(500).json({ success: false, error: err.message, creditsLost: true, requestId });
        setTimeout(() => process.exit(1), 500);
        return;
      } else {
        log('✅', label, `Credits unchanged (${creditsNow}) — panel hiccup, staying alive`);
      }
    } catch (creditsErr) {
      log('⚠️', label, `Could not check credits after failure`, { error: creditsErr.message });
    }

    res.status(500).json({ success: false, error: err.message, requestId });
  }
});

router.post('/mag/renew', async (req, res) => {
  const { id, package: packageId, note } = req.body;
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-MAGRenew:${requestId}`;

  log('📥', label, `MAG renew request received`, { id, packageId });

  if (!id || typeof id !== 'number') {
    return res.status(400).json({ success: false, error: 'id must be a number', requestId });
  }
  if (!packageId || typeof packageId !== 'number') {
    return res.status(400).json({ success: false, error: 'package must be a number', requestId });
  }
  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive. Check /health.', requestId });
  }

  try {
    await withRetry(() => renewSubscription(id, packageId, note), label);
    log('🎉', label, `MAG renew complete`, { id, packageId });
    res.json({ success: true, message: `MAG subscription ${id} renewed with package ${packageId}`, subscriptionId: id, requestId });
  } catch (err) {
    log('❌', label, `MAG renew failed`, { error: err.message });

    const creditsBefore = credits.current;
    try {
      const creditsNow = await fetchCredits();
      log('💰', label, `Credits check after failure`, { before: creditsBefore, after: creditsNow });
      if (creditsBefore !== null && creditsNow !== null && creditsNow < creditsBefore) {
        log('🛑', label, `Credits decreased (${creditsBefore} → ${creditsNow}) — shutting down to prevent further loss`);
        res.status(500).json({ success: false, error: err.message, creditsLost: true, requestId });
        setTimeout(() => process.exit(1), 500);
        return;
      } else {
        log('✅', label, `Credits unchanged (${creditsNow}) — panel hiccup, staying alive`);
      }
    } catch (creditsErr) {
      log('⚠️', label, `Could not check credits after failure`, { error: creditsErr.message });
    }

    res.status(500).json({ success: false, error: err.message, requestId });
  }
});

router.get('/mag/:mac', async (req, res) => {
  const { mac } = req.params;
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-MAGGet:${requestId}`;

  log('📥', label, `MAG get request`, { mac });

  const normalizedMac = normalizeMac(mac);
  if (!normalizedMac) {
    return res.status(400).json({ success: false, error: `Invalid MAC address format: "${mac}"`, requestId });
  }
  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive', requestId });
  }

  try {
    const subscription = await withRetry(() => findMagSubscription(normalizedMac, 3), label);
    res.json({ success: true, subscription, requestId });
  } catch (err) {
    log('❌', label, `MAG not found`, { mac: normalizedMac, error: err.message });
    res.status(404).json({ success: false, error: err.message, requestId });
  }
});

router.get('/mags', async (req, res) => {
  const requestId = crypto.randomBytes(8).toString('hex');
  const label = `API-MAGList:${requestId}`;

  log('📥', label, `MAG list request`);
  if (!session.alive) {
    return res.status(503).json({ success: false, error: 'Session is not alive', requestId });
  }

  try {
    const data = await withRetry(() => fetchMagSubscriptions(), label);
    const subscriptions = data.data.map(extractMagSubFields);
    log('✅', label, `Listed ${subscriptions.length} MAG subscriptions`);
    res.json({ success: true, total: data.recordsTotal, filtered: data.recordsFiltered, count: subscriptions.length, subscriptions, requestId });
  } catch (err) {
    log('❌', label, `MAG list failed`, { error: err.message });
    res.status(500).json({ success: false, error: err.message, requestId });
  }
});

// ─── STATUS / CONFIG / EXPORTS ───────────────────────────────────────────────

function getStatus() {
  const uptime = Math.floor((Date.now() - session.startTime) / 1000);
  return {
    key: PANEL_KEY,
    name: PANEL_NAME,
    type: 'session',
    alive: session.alive,
    uptimeSeconds: uptime,
    lastCheck: session.lastCheck,
    errors: session.errors,
    consecutiveErrors: session.consecutiveErrors,
    lastError: null,
    info: {
      baseUrl: CONFIG.BASE_URL,
      resellerId: CONFIG.RESELLER_ID,
      keepaliveInterval: CONFIG.KEEPALIVE_INTERVAL,
      tokenAge: tokenAge(),
      tokenRefreshes: tokens.refreshCount,
      credits: credits.current,
    },
    stats,
  };
}

// Dashboard hook: this panel authenticates with cookies + a CSRF token. Accept
// either individual fields or a raw browser Cookie string (paste from DevTools).
async function configure(payload = {}) {
  const changed = [];

  // Raw "name=value; name2=value2" cookie string → extract the two we need.
  if (typeof payload.cookies === 'string' && payload.cookies.trim()) {
    const jar = {};
    payload.cookies.split(';').forEach(pair => {
      const idx = pair.indexOf('=');
      if (idx > 0) jar[pair.slice(0, idx).trim()] = pair.slice(idx + 1).trim();
    });
    if (jar['XSRF-TOKEN'])     { tokens.xsrfToken = jar['XSRF-TOKEN']; changed.push('XSRF-TOKEN'); }
    if (jar['laravel_session']){ tokens.managementSession = jar['laravel_session']; changed.push('laravel_session'); }
  }

  if (typeof payload.xsrfToken === 'string' && payload.xsrfToken.length)       { tokens.xsrfToken = payload.xsrfToken; changed.push('XSRF-TOKEN'); }
  if (typeof payload.laravelSession === 'string' && payload.laravelSession.length) { tokens.managementSession = payload.laravelSession; changed.push('laravel_session'); }
  if (typeof payload.csrfToken === 'string' && payload.csrfToken.length)       { CONFIG.CSRF_TOKEN = payload.csrfToken; changed.push('csrf-token'); }

  if (!changed.length) {
    return { success: false, error: 'Nothing to update. Provide cookies (raw string) and/or xsrfToken/laravelSession/csrfToken.' };
  }

  tokens.lastRefresh = new Date();
  const alive = await validateSession();
  return { success: alive, changed, alive };
}

module.exports = {
  meta: { key: PANEL_KEY, name: PANEL_NAME, type: 'session' },
  router,
  startKeepAlive,
  getStatus,
  configure,
};
