-- Disposable synthetic qualification harness only.
-- The application uses hapi_test; independent observations use this SELECT-only role.
CREATE ROLE hapi_observer LOGIN PASSWORD 'local-observer-only-not-a-secret';
GRANT CONNECT ON DATABASE hapi TO hapi_observer;
GRANT USAGE ON SCHEMA public TO hapi_observer;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO hapi_observer;
ALTER DEFAULT PRIVILEGES FOR ROLE hapi_test IN SCHEMA public
  GRANT SELECT ON TABLES TO hapi_observer;
ALTER ROLE hapi_observer SET default_transaction_read_only = on;
