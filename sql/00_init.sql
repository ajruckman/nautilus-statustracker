CREATE DATABASE nautilus;

\connect nautilus

SELECT current_database();

CREATE USER statustracker_mgr WITH ENCRYPTED PASSWORD '29beb933113f899c3613';
CREATE USER statustracker_usr WITH ENCRYPTED PASSWORD 'df3009c0240b9ef02b20';
CREATE USER statustracker_ro  WITH ENCRYPTED PASSWORD '3d354917796cc70a5347';

GRANT statustracker_usr TO statustracker_mgr;
GRANT statustracker_ro  TO statustracker_usr;
GRANT statustracker_mgr TO db1;

GRANT CONNECT ON DATABASE nautilus TO statustracker_ro; -- others inherit

-- Create new schema
CREATE SCHEMA statustracker AUTHORIZATION statustracker_mgr;

SET search_path = statustracker;

-- These are not inheritable
ALTER ROLE statustracker_mgr IN DATABASE nautilus SET search_path = statustracker;
ALTER ROLE statustracker_usr IN DATABASE nautilus SET search_path = statustracker;
ALTER ROLE statustracker_ro  IN DATABASE nautilus SET search_path = statustracker;

GRANT CREATE ON SCHEMA statustracker TO statustracker_mgr;
GRANT USAGE  ON SCHEMA statustracker TO statustracker_ro ; -- statustracker_usr inherits

-- Set default privileges
-- -> Read only
ALTER DEFAULT PRIVILEGES FOR ROLE statustracker_mgr GRANT SELECT ON TABLES TO statustracker_ro;

-- -> Read/write
ALTER DEFAULT PRIVILEGES FOR ROLE statustracker_mgr GRANT INSERT, UPDATE, DELETE, TRUNCATE ON TABLES TO statustracker_usr;

-- -> Read/write for sequences
ALTER DEFAULT PRIVILEGES FOR ROLE statustracker_mgr GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO statustracker_usr;
