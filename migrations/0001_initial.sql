-- Mirrors the manually created production D1 schema (sqlite_master, 2026-09-24).
-- CREATE ... IF NOT EXISTS leaves the existing remote tables and indexes untouched.
CREATE TABLE IF NOT EXISTS security_stat (
    trade_date TEXT NOT NULL,
    market_code TEXT NOT NULL,
    orderbook_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    market_segment TEXT,
    trading_currency TEXT,
    financial_product TEXT,
    sector_code TEXT,
    previous_close REAL,
    open REAL,
    high REAL,
    low REAL,
    last_traded_price REAL,
    average_price REAL,
    best_bid REAL,
    best_offer REAL,
    turnover_quantity REAL,
    turnover_value REAL,
    total_trade INTEGER,
    trade_report_quantity REAL,
    trade_report_value REAL,
    short_sell_quantity REAL,
    short_sell_value REAL,
    par REAL,
    lot_size INTEGER,
    listed_shares INTEGER,
    book_value REAL,
    eps REAL,
    dps REAL,
    pe REAL,
    pbv REAL,
    dividend_yield REAL,
    market_cap REAL,
    corporate_action_code TEXT,
    notification_sign TEXT,
    other_sign TEXT,
    isin TEXT,
    isin_nvdr TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (trade_date, market_code, orderbook_id)
);
CREATE INDEX IF NOT EXISTS idx_security_stat_symbol_date ON security_stat (symbol, trade_date);

CREATE TABLE IF NOT EXISTS index_stat (
    trade_date TEXT NOT NULL,
    data_round TEXT,
    market_segment TEXT,
    industry_code TEXT,
    sector_code TEXT,
    index_name TEXT NOT NULL,
    index_value REAL,
    previous_close REAL,
    open REAL,
    high REAL,
    low REAL,
    change REAL,
    volume REAL,
    value REAL,
    pe REAL,
    pbv REAL,
    dividend_yield REAL,
    market_cap REAL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (trade_date, index_name)
);
CREATE INDEX IF NOT EXISTS idx_index_stat_name_date ON index_stat (index_name, trade_date);

CREATE TABLE IF NOT EXISTS market_stat (
    trade_date TEXT NOT NULL,
    data_round TEXT,
    market_stat_id TEXT NOT NULL,       -- SET / MAI
    trading_currency TEXT NOT NULL,     -- currently THB
    total_trades INTEGER,
    total_quantity INTEGER,
    total_value INTEGER,
    up_quantity INTEGER,
    down_quantity INTEGER,
    no_change_quantity INTEGER,
    up_shares INTEGER,
    down_shares INTEGER,
    no_change_shares INTEGER,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (trade_date, market_stat_id, trading_currency)
);
CREATE INDEX IF NOT EXISTS idx_market_stat_market_date ON market_stat (market_stat_id, trade_date);

CREATE TABLE IF NOT EXISTS investor_stat (
    trade_date TEXT NOT NULL,
    data_round TEXT,
    period_type TEXT NOT NULL,        -- DAILY / MTD
    market_segment TEXT NOT NULL,     -- SET / MAI
    trading_currency TEXT NOT NULL,   -- THB / USD
    buy_institute_value REAL,
    buy_foreign_value REAL,
    buy_customer_value REAL,
    buy_proprietary_value REAL,
    sell_institute_value REAL,
    sell_foreign_value REAL,
    sell_customer_value REAL,
    sell_proprietary_value REAL,
    total_value REAL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (trade_date, period_type, market_segment, trading_currency)
);
CREATE INDEX IF NOT EXISTS idx_investor_stat_market_date ON investor_stat (market_segment, trade_date);

CREATE TABLE IF NOT EXISTS security_update (
    report_date TEXT NOT NULL,
    data_round TEXT,
    market_code TEXT NOT NULL,
    market_segment TEXT,
    trading_currency TEXT,
    orderbook_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    long_name TEXT,
    originates_from TEXT,
    financial_product TEXT,
    sector_code TEXT,
    pqf REAL,
    par REAL,
    lot_size INTEGER,
    isin TEXT,
    isin_nvdr TEXT,
    instrument_state TEXT,
    instrument_status TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (report_date, market_code, orderbook_id)
);
CREATE INDEX IF NOT EXISTS idx_security_update_symbol_date ON security_update (symbol, report_date);

-- No natural key: the writer skips rows that exactly match an existing row.
CREATE TABLE IF NOT EXISTS corporate_action (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    orderbook_id INTEGER,
    event_type TEXT NOT NULL,          -- XD / XR / XW / SPLIT / DIVIDEND / RIGHTS
    announcement_date TEXT,
    ex_date TEXT,
    record_date TEXT,
    payment_date TEXT,
    effective_date TEXT,
    cash_dividend REAL,
    stock_dividend_ratio REAL,
    split_ratio REAL,
    rights_ratio REAL,
    rights_price REAL,
    old_par REAL,
    new_par REAL,
    old_symbol TEXT,
    new_symbol TEXT,
    currency TEXT,
    source TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_corporate_action_symbol_date ON corporate_action (symbol, ex_date);

-- Append-only: one row per attempt.
CREATE TABLE IF NOT EXISTS ingestion_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trade_date TEXT NOT NULL,
    dataset TEXT NOT NULL,        -- security_stat / index_stat / market_stat / ...
    source TEXT,                  -- yahoo / set_public / set_oaq
    status TEXT NOT NULL,         -- SUCCESS / FAILED / PARTIAL
    row_count INTEGER DEFAULT 0,
    started_at TEXT,
    finished_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_ingestion_log_date_dataset ON ingestion_log (trade_date, dataset);
