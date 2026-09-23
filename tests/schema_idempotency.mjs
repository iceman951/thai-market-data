import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { DatabaseSync } from 'node:sqlite';

const db = new DatabaseSync(':memory:');
const schema = readFileSync(new URL('../migrations/0001_initial.sql', import.meta.url), 'utf8');
db.exec(schema);
db.exec(schema);

const count = table => db.prepare(`SELECT count(*) AS n FROM ${table}`).get().n;
const primaryKey = table => db.prepare(`PRAGMA table_info(${table})`).all()
  .filter(c => c.pk > 0).sort((a, b) => a.pk - b.pk).map(c => c.name);

// Local keys must also be checked against production before deployment.
assert.deepEqual(primaryKey('security_stat'), ['trade_date', 'market_code', 'orderbook_id']);
assert.deepEqual(primaryKey('index_stat'), ['trade_date', 'index_name']);
assert.deepEqual(primaryKey('market_stat'), ['trade_date', 'market_stat_id', 'trading_currency']);
assert.deepEqual(primaryKey('investor_stat'), ['trade_date', 'period_type', 'market_segment', 'trading_currency']);
assert.deepEqual(primaryKey('security_update'), ['report_date', 'market_code', 'orderbook_id']);
assert.deepEqual(primaryKey('corporate_action'), ['id']);
assert.deepEqual(primaryKey('ingestion_log'), ['id']);

// Upserts keep one row per key and preserve created_at.
const upserts = [
  ['security_stat', 'INSERT INTO security_stat (trade_date,market_code,orderbook_id,symbol,open) VALUES (?,?,?,?,?) ON CONFLICT(trade_date,market_code,orderbook_id) DO UPDATE SET symbol=excluded.symbol,open=excluded.open', ['2026-09-24', 'XBKK', 1, 'MOCK']],
  ['index_stat', 'INSERT INTO index_stat (trade_date,index_name,index_value) VALUES (?,?,?) ON CONFLICT(trade_date,index_name) DO UPDATE SET index_value=excluded.index_value', ['2026-09-24', 'SET']],
  ['market_stat', 'INSERT INTO market_stat (trade_date,market_stat_id,trading_currency,total_value) VALUES (?,?,?,?) ON CONFLICT(trade_date,market_stat_id,trading_currency) DO UPDATE SET total_value=excluded.total_value', ['2026-09-24', 'SET', 'THB']],
  ['investor_stat', 'INSERT INTO investor_stat (trade_date,period_type,market_segment,trading_currency,total_value) VALUES (?,?,?,?,?) ON CONFLICT(trade_date,period_type,market_segment,trading_currency) DO UPDATE SET total_value=excluded.total_value', ['2026-09-24', 'DAILY', 'SET', 'THB']],
  ['security_update', 'INSERT INTO security_update (report_date,market_code,orderbook_id,symbol,par) VALUES (?,?,?,?,?) ON CONFLICT(report_date,market_code,orderbook_id) DO UPDATE SET symbol=excluded.symbol,par=excluded.par', ['2026-09-24', 'XBKK', 1, 'MOCK']],
];
for (const [table, sql, key] of upserts) {
  const statement = db.prepare(sql);
  statement.run(...key, 1.5);
  const createdAt = db.prepare(`SELECT created_at FROM ${table}`).get().created_at;
  statement.run(...key, 2.5);
  assert.equal(count(table), 1, table);
  assert.equal(db.prepare(`SELECT created_at FROM ${table}`).get().created_at, createdAt, table);
}

// corporate_action has no natural key; the writer skips exact duplicates, NULLs included.
const action = db.prepare('INSERT INTO corporate_action (symbol,event_type,ex_date,cash_dividend) SELECT ?1,?2,?3,?4 WHERE NOT EXISTS (SELECT 1 FROM corporate_action WHERE symbol IS ?1 AND event_type IS ?2 AND ex_date IS ?3 AND cash_dividend IS ?4)');
for (let i = 0; i < 2; i++) {
  action.run('MOCK', 'XD', null, 0.5);
  action.run('MOCK', 'XD', '2026-10-15', 0.5);
}
assert.equal(count('corporate_action'), 2);
action.run('MOCK', 'XD', '2026-10-15', 0.6);
assert.equal(count('corporate_action'), 3);

// ingestion_log is append-only.
const log = db.prepare("INSERT INTO ingestion_log (trade_date,dataset,source,status,row_count,started_at,finished_at,error_message) VALUES ('2026-09-24','index_stat','mock','SUCCESS',1,'t0','t1',NULL)");
log.run();
log.run();
assert.equal(count('ingestion_log'), 2);

console.log('schema and idempotent inserts: ok');
