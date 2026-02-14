-- Seed Data: Free and Paid Data Sources
-- Comprehensive catalog of financial and economic data providers

-- ============================================
-- FREE DATA SOURCES
-- ============================================

-- 1. ALPHA VANTAGE (Free/Freemium)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, api_version, documentation_url,
    auth_type, api_key_env_var,
    rate_limit_requests, rate_limit_window,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111111',
    'Alpha Vantage',
    'Free stock API for realtime and historical market data, options, forex, commodity, and cryptocurrency feeds',
    'alpha_vantage',
    'market_data',
    'freemium',
    'https://www.alphavantage.co',
    'v1',
    'https://www.alphavantage.co/documentation/',
    'api_key',
    'ALPHA_VANTAGE_API_KEY',
    25,
    'day',
    'active',
    true,
    10,
    '{"premium_tiers": true, "websocket": false}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description, required_params) VALUES
('11111111-1111-1111-1111-111111111111', 'Time Series Intraday', 'GET', '/query', 'Intraday time series data', '["function", "symbol", "interval"]'::jsonb),
('11111111-1111-1111-1111-111111111111', 'Time Series Daily', 'GET', '/query', 'Daily time series data', '["function", "symbol"]'::jsonb),
('11111111-1111-1111-1111-111111111111', 'Quote Endpoint', 'GET', '/query', 'Global quote for equity', '["function", "symbol"]'::jsonb),
('11111111-1111-1111-1111-111111111111', 'Search', 'GET', '/query', 'Symbol search', '["function", "keywords"]'::jsonb),
('11111111-1111-1111-1111-111111111111', 'Fundamentals Overview', 'GET', '/query', 'Company overview', '["function", "symbol"]'::jsonb);

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, requests_per_day, requests_per_minute, features) VALUES
('11111111-1111-1111-1111-111111111111', 'Free', 1, 0, 0, 25, 5, '["historical", "intraday"]'::jsonb),
('11111111-1111-1111-1111-111111111111', 'Premium', 2, 49.99, 599.88, NULL, 75, '["realtime", "fundamentals", "options"]'::jsonb);

-- 2. YAHOO FINANCE (Free)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111112',
    'Yahoo Finance',
    'Free financial data including historical prices, fundamentals, and market data (unofficial API via yfinance)',
    'yahoo_finance',
    'market_data',
    'free',
    'https://finance.yahoo.com',
    'https://github.com/ranaroussi/yfinance',
    'none',
    'active',
    true,
    5,
    '{"unofficial_api": true, "rate_limit": "2000/hour"}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111112', 'Historical Data', 'GET', '/history', 'Historical OHLCV data'),
('11111111-1111-1111-1111-111111111112', 'Ticker Info', 'GET', '/info', 'Company information and fundamentals'),
('11111111-1111-1111-1111-111111111112', 'Financials', 'GET', '/financials', 'Financial statements'),
('11111111-1111-1111-1111-111111111112', 'Options Chain', 'GET', '/options', 'Options chain data');

-- 3. FRED (Free - St. Louis Fed)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, api_version, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111113',
    'FRED Economic Data',
    'Federal Reserve Economic Data - 800,000+ economic time series from 100+ sources',
    'fred',
    'economic',
    'free',
    'https://api.stlouisfed.org',
    'v1',
    'https://fred.stlouisfed.org/docs/api/fred/',
    'api_key',
    'FRED_API_KEY',
    'active',
    true,
    8,
    '{"series_count": 800000}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111113', 'Series Observations', 'GET', '/fred/series/observations', 'Get data series observations'),
('11111111-1111-1111-1111-111111111113', 'Series Info', 'GET', '/fred/series', 'Get series information'),
('11111111-1111-1111-1111-111111111113', 'Category', 'GET', '/fred/category', 'Get category information'),
('11111111-1111-1111-1111-111111111113', 'Release', 'GET', '/fred/release', 'Get release information'),
('11111111-1111-1111-1111-111111111113', 'Search', 'GET', '/fred/series/search', 'Search for series');

-- 4. WORLD BANK OPEN DATA (Free)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111114',
    'World Bank Open Data',
    'Free access to global development data from the World Bank - 3000+ indicators, 266 countries',
    'world_bank',
    'economic',
    'free',
    'https://api.worldbank.org/v2',
    'https://datahelpdesk.worldbank.org/knowledgebase/articles/889386-api-faqs',
    'none',
    'active',
    true,
    15,
    '{"indicators": 3000, "countries": 266}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111114', 'Country Indicator', 'GET', '/country/{country}/indicator/{indicator}', 'Get indicator data for country'),
('11111111-1111-1111-1111-111111111114', 'Countries', 'GET', '/country', 'List all countries'),
('11111111-1111-1111-1111-111111111114', 'Indicators', 'GET', '/indicator', 'List all indicators'),
('11111111-1111-1111-1111-111111111114', 'Income Levels', 'GET', '/incomeLevel', 'List income levels');

-- 5. EUROSTAT (Free)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111115',
    'Eurostat',
    'Statistical office of the European Union - official statistics on EU member states',
    'eurostat',
    'economic',
    'free',
    'https://ec.europa.eu/eurostat/api/dissemination',
    'https://ec.europa.eu/eurostat/web/main/data/web-services',
    'none',
    'active',
    true,
    12,
    '{"formats": ["JSON", "SDMX"]}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111115', 'Statistics', 'GET', '/statistics/1.0/data/{datasetCode}', 'Get dataset data'),
('11111111-1111-1111-1111-111111111115', 'Datasets', 'GET', '/catalogue/1.0/datasets', 'List available datasets');

-- 6. COINGECKO (Free/Freemium)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, api_version, documentation_url,
    auth_type,
    rate_limit_requests, rate_limit_window,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111116',
    'CoinGecko',
    'Cryptocurrency data API - prices, market cap, volume, exchange data for 13,000+ coins',
    'coingecko',
    'crypto',
    'freemium',
    'https://api.coingecko.com',
    'v3',
    'https://www.coingecko.com/en/api/documentation',
    'none',
    50,
    'minute',
    'active',
    true,
    7,
    '{"coins": 13000, "exchanges": 800}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111116', 'Coins List', 'GET', '/coins/list', 'List all supported coins'),
('11111111-1111-1111-1111-111111111116', 'Coin Markets', 'GET', '/coins/markets', 'Coin prices and market data'),
('11111111-1111-1111-1111-111111111116', 'Coin History', 'GET', '/coins/{id}/history', 'Historical data for coin'),
('11111111-1111-1111-1111-111111111116', 'Exchanges', 'GET', '/exchanges', 'List all exchanges'),
('11111111-1111-1111-1111-111111111116', 'Trending', 'GET', '/search/trending', 'Top 7 trending coins');

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, requests_per_minute, features) VALUES
('11111111-1111-1111-1111-111111111116', 'Free', 1, 0, 0, 30, '["public", "pro_plan_limited"]'::jsonb),
('11111111-1111-1111-1111-111111111116', 'Analyst', 2, 129, 1290, 500, '["all_endpoints", "priority_support"]'::jsonb),
('11111111-1111-1111-1111-111111111116', 'Enterprise', 3, 599, 5990, NULL, '["unlimited", "dedicated_support"]'::jsonb);

-- 7. NEWSAPI (Free/Freemium)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, api_version, documentation_url,
    auth_type, api_key_env_var,
    rate_limit_requests, rate_limit_window,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111117',
    'NewsAPI',
    'Headlines from 30,000 news sources in 50+ countries',
    'newsapi',
    'news',
    'freemium',
    'https://newsapi.org',
    'v2',
    'https://newsapi.org/docs',
    'api_key',
    'NEWSAPI_KEY',
    100,
    'day',
    'active',
    true,
    20,
    '{"sources": 30000, "countries": 50}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111117', 'Top Headlines', 'GET', '/v2/top-headlines', 'Top headlines'),
('11111111-1111-1111-1111-111111111117', 'Everything', 'GET', '/v2/everything', 'Search all articles'),
('11111111-1111-1111-1111-111111111117', 'Sources', 'GET', '/v2/sources', 'News sources list');

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, requests_per_day, features) VALUES
('11111111-1111-1111-1111-111111111117', 'Developer', 1, 0, 0, 100, '["news", "blog"]'::jsonb),
('11111111-1111-1111-1111-111111111117', 'Business', 2, 449, 4490, NULL, '["historical", "sentiment", "priority"]'::jsonb);

-- 8. OPENWEATHERMAP (Free/Freemium)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, api_version, documentation_url,
    auth_type, api_key_env_var,
    rate_limit_requests, rate_limit_window,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111118',
    'OpenWeatherMap',
    'Weather data API - current weather, forecasts, historical data',
    'openweathermap',
    'geospatial',
    'freemium',
    'https://api.openweathermap.org',
    '2.5',
    'https://openweathermap.org/api',
    'api_key',
    'OPENWEATHER_API_KEY',
    1000,
    'day',
    'active',
    true,
    30,
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111118', 'Current Weather', 'GET', '/data/2.5/weather', 'Current weather data'),
('11111111-1111-1111-1111-111111111118', 'Forecast', 'GET', '/data/2.5/forecast', '5 day forecast'),
('11111111-1111-1111-1111-111111111118', 'Air Pollution', 'GET', '/data/2.5/air_pollution', 'Air quality index');

-- 9. EIA OPEN DATA (Free)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, api_version, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-111111111119',
    'EIA Open Data',
    'US Energy Information Administration - energy statistics and data',
    'eia',
    'commodities',
    'free',
    'https://api.eia.gov',
    'v2',
    'https://www.eia.gov/opendata/',
    'api_key',
    'EIA_API_KEY',
    'active',
    true,
    18,
    '{"categories": ["petroleum", "natural_gas", "electricity", "coal"]}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-111111111119', 'Series', 'GET', '/v2/series', 'Get data series'),
('11111111-1111-1111-1111-111111111119', 'Facets', 'GET', '/v2/facets', 'Get available facets'),
('11111111-1111-1111-1111-111111111119', 'Geoset', 'GET', '/v2/geoset', 'Geographic data sets');

-- 10. DBNOMICS (Free)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-11111111111a',
    'DBnomics',
    'Aggregator of economic data from 100+ providers (Fed, ECB, BIS, OECD, etc.)',
    'dbnomics',
    'economic',
    'free',
    'https://api.db.nomics.world/v22',
    'https://db.nomics.world/api-docs',
    'none',
    'active',
    true,
    6,
    '{"providers": 100}'::jsonb
) ON CONFLICT DO NOTHING;

-- ============================================
-- PAID DATA SOURCES (Pricing Catalog)
-- ============================================

-- 11. POLYGON.IO
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111111',
    'Polygon.io',
    'Real-time and historical market data for US stocks, options, forex, and crypto',
    'polygon',
    'market_data',
    'paid',
    'https://api.polygon.io',
    'https://polygon.io/docs',
    'api_key',
    'POLYGON_API_KEY',
    'inactive',
    false,
    3,
    '{"realtime": true, "websocket": true}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, features) VALUES
('21111111-1111-1111-1111-111111111111', 'Stocks Starter', 2, 199, 1990, '["realtime", "us_equities", "options"]'::jsonb),
('21111111-1111-1111-1111-111111111111', 'Stocks Advanced', 3, 399, 3990, '["advanced_options", "tick_data"]'::jsonb),
('21111111-1111-1111-1111-111111111111', 'All Asset Classes', 4, 799, 7990, '["forex", "crypto", "global"]'::jsonb);

-- 12. IEX CLOUD
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111112',
    'IEX Cloud',
    'US stock market data - real-time, historical, fundamentals',
    'iex',
    'market_data',
    'freemium',
    'https://cloud.iexapis.com',
    'https://iexcloud.io/docs/api/',
    'api_key',
    'IEX_CLOUD_API_KEY',
    'inactive',
    false,
    9,
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, requests_per_month, features) VALUES
('21111111-1111-1111-1111-111111111112', 'Launch', 1, 0, 0, 50000, '["core_data", "stock_prices"]'::jsonb),
('21111111-1111-1111-1111-111111111112', 'Grow', 2, 199, 1990, 5000000, '["intraday", "fundamentals"]'::jsonb),
('21111111-1111-1111-1111-111111111112', 'Scale', 3, 599, 5990, NULL, '["everything", "priority"]'::jsonb);

-- 13. FINANCIAL MODELING PREP
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111113',
    'Financial Modeling Prep',
    'Financial statement data, ratios, stock fundamentals',
    'fmp',
    'market_data',
    'freemium',
    'https://financialmodelingprep.com/api',
    'https://site.financialmodelingprep.com/developer/docs/',
    'api_key',
    'FMP_API_KEY',
    'inactive',
    false,
    11,
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, requests_per_day, features) VALUES
('21111111-1111-1111-1111-111111111113', 'Free', 1, 0, 0, 250, '["limited_endpoints"]'::jsonb),
('21111111-1111-1111-1111-111111111113', 'Starter', 2, 228, 2280, NULL, '["all_data", "websocket"]'::jsonb),
('21111111-1111-1111-1111-111111111113', 'Professional', 3, 828, 8280, NULL, '["premium", "institutional"]'::jsonb);

-- 14. BLOOMBERG TERMINAL
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111114',
    'Bloomberg Terminal',
    'Institutional-grade financial data terminal - real-time markets, news, analytics',
    'bloomberg',
    'market_data',
    'paid',
    'https://www.bloomberg.com/professional/',
    'https://www.bloomberg.com/professional/support/api-library/',
    'oauth2',
    'inactive',
    false,
    1,
    '{"terminal_required": true, "institutional": true}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_yearly_usd, features) VALUES
('21111111-1111-1111-1111-111111111114', 'Terminal', 5, 24000, '["everything", "realtime", "news", "analytics", "messaging"]'::jsonb),
('21111111-1111-1111-1111-111111111114', 'API Only', 4, 2400, '["data_feed", "no_terminal"]'::jsonb);

-- 15. REFINITIV EIKON
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111115',
    'Refinitiv Eikon',
    'Financial markets data and news from LSEG (London Stock Exchange Group)',
    'refinitiv',
    'market_data',
    'paid',
    'https://developers.refinitiv.com/',
    'https://developers.refinitiv.com/en/api-catalog',
    'oauth2',
    'inactive',
    false,
    2,
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_yearly_usd, features) VALUES
('21111111-1111-1111-1111-111111111115', 'Entry', 2, 3600, '["delayed", "basics"]'::jsonb),
('21111111-1111-1111-1111-111111111115', 'Professional', 3, 12000, '["realtime", "news", "analytics"]'::jsonb),
('21111111-1111-1111-1111-111111111115', 'Enterprise', 4, 22000, '["everything", "api_access"]'::jsonb);

-- 16. FACTSET
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111116',
    'FactSet',
    'Financial data and analytics for portfolio management and research',
    'factset',
    'market_data',
    'paid',
    'https://www.factset.com/',
    'https://developer.factset.com/',
    'oauth2',
    'inactive',
    false,
    4,
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_yearly_usd, features) VALUES
('21111111-1111-1111-1111-111111111116', 'Standard', 3, 12000, '["analytics", "screening"]'::jsonb),
('21111111-1111-1111-1111-111111111116', 'Enterprise', 4, 25000, '["full_suite", "api"]'::jsonb);

-- 17. TIINGO
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111117',
    'Tiingo',
    'Historical and real-time stock prices, news, and fundamentals',
    'tiingo',
    'market_data',
    'freemium',
    'https://api.tiingo.com',
    'https://www.tiingo.com/documentation/',
    'api_key',
    'TIINGO_API_KEY',
    'inactive',
    false,
    13,
    '{}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, requests_per_day, features) VALUES
('21111111-1111-1111-1111-111111111117', 'Free', 1, 0, 0, 500, '["end_of_day", "news"]'::jsonb),
('21111111-1111-1111-1111-111111111117', 'Power', 2, 348, 3480, 50000, '["intraday", "fundamentals", "crypto"]'::jsonb),
('21111111-1111-1111-1111-111111111117', 'Commercial', 3, 1000, 10000, 100000, '["websocket", "support"]'::jsonb);

-- 18. EODHD
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111118',
    'EODHD',
    'Global market data - stocks, ETFs, forex, crypto from 70+ exchanges',
    'eodhd',
    'market_data',
    'freemium',
    'https://eodhistoricaldata.com/api',
    'https://eodhistoricaldata.com/financial-apis-blog/',
    'api_key',
    'EODHD_API_KEY',
    'inactive',
    false,
    14,
    '{"exchanges": 70}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_monthly_usd, price_yearly_usd, requests_per_day, features) VALUES
('21111111-1111-1111-1111-111111111118', 'Free', 1, 0, 0, 20, '["limited_symbols"]'::jsonb),
('21111111-1111-1111-1111-111111111118', 'Starter', 2, 19, 190, 100000, '["all_symbols", "fundamentals"]'::jsonb),
('21111111-1111-1111-1111-111111111118', 'Pro', 3, 49, 490, 1000000, '["intraday", "options"]'::jsonb),
('21111111-1111-1111-1111-111111111118', 'Enterprise', 4, 199, 1990, NULL, '["unlimited", "api"]'::jsonb);

-- 19. RAVENPACK
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '21111111-1111-1111-1111-111111111119',
    'RavenPack',
    'News analytics and sentiment data for financial markets',
    'ravenpack',
    'news',
    'paid',
    'https://www.ravenpack.com/',
    'https://www.ravenpack.com/products/api',
    'oauth2',
    'inactive',
    false,
    16,
    '{"sentiment": true, "entity_detection": true}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_pricing (source_id, tier_name, tier_level, price_yearly_usd, features) VALUES
('21111111-1111-1111-1111-111111111119', 'Data License', 4, NULL, '["historical", "realtime", "analytics"]'::jsonb);

-- 20. DEFILLAMA (Free)
INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type,
    status, is_enabled, priority,
    config
) VALUES (
    '11111111-1111-1111-1111-11111111111b',
    'DeFiLlama',
    'DeFi TVL aggregator - total value locked across all DeFi protocols',
    'defillama',
    'crypto',
    'free',
    'https://api.llama.fi',
    'https://defillama.com/docs/api',
    'none',
    'active',
    true,
    19,
    '{"protocols": 2000, "chains": 150}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('11111111-1111-1111-1111-11111111111b', 'Protocols', 'GET', '/protocols', 'List all protocols'),
('11111111-1111-1111-1111-11111111111b', 'TVL', 'GET', '/tvl/{protocol}', 'Get TVL for protocol'),
('11111111-1111-1111-1111-11111111111b', 'Chains', 'GET', '/chains', 'List all chains');

-- ============================================
-- WEB SCRAPER SOURCE (Firecrawl)
-- ============================================

INSERT INTO data_sources (
    id, name, description, provider, category, source_type, 
    base_url, documentation_url,
    auth_type, api_key_env_var,
    status, is_enabled, priority,
    config
) VALUES (
    '31111111-1111-1111-1111-111111111111',
    'Firecrawl (Self-Hosted)',
    'Web scraping for LLMs - extract structured data from any website',
    'firecrawl',
    'alternative',
    'scraped',
    'http://localhost:3002',
    'https://docs.firecrawl.dev/',
    'api_key',
    'FIRECRAWL_API_KEY',
    'inactive',
    false,
    21,
    '{"self_hosted": true, "formats": ["markdown", "html", "json"]}'::jsonb
) ON CONFLICT DO NOTHING;

INSERT INTO data_source_endpoints (source_id, name, method, path, description) VALUES
('31111111-1111-1111-1111-111111111111', 'Scrape', 'POST', '/v1/scrape', 'Scrape single URL'),
('31111111-1111-1111-1111-111111111111', 'Crawl', 'POST', '/v1/crawl', 'Crawl entire website'),
('31111111-1111-1111-1111-111111111111', 'Map', 'POST', '/v1/map', 'Map all URLs on website'),
('31111111-1111-1111-1111-111111111111', 'Search', 'POST', '/v1/search', 'Search and scrape');

-- Update the updated_at column
UPDATE data_sources SET updated_at = NOW();
