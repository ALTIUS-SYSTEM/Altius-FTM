-- Dispatch fields the task editor has always collected but never stored.
--
-- The web editor offered Flow, Date, Start time, Priority and Instructions;
-- none of them reached the API, so an operator's driver instructions were
-- silently dropped on save. `day` already existed but was written from
-- created_at, so choosing a date did nothing either.
--
-- All nullable except priority: existing rows predate the fields and have no
-- meaningful value to backfill, and "unknown" is honest for them.

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS flow       TEXT;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS start_time TEXT;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS notes      TEXT;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS priority   TEXT NOT NULL DEFAULT 'normal';

-- Bounds the API also enforces, repeated here so a direct SQL write or a
-- second client cannot store a shape the readers do not expect.
ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_priority_check;
ALTER TABLE tasks ADD CONSTRAINT tasks_priority_check
    CHECK (priority IN ('normal', 'high'));

ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_start_time_check;
ALTER TABLE tasks ADD CONSTRAINT tasks_start_time_check
    CHECK (start_time IS NULL OR start_time ~ '^[0-2][0-9]:[0-5][0-9]$');

ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_day_check;
ALTER TABLE tasks ADD CONSTRAINT tasks_day_check
    CHECK (day ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$');

ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_flow_len_check;
ALTER TABLE tasks ADD CONSTRAINT tasks_flow_len_check
    CHECK (flow IS NULL OR char_length(flow) <= 64);

ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_notes_len_check;
ALTER TABLE tasks ADD CONSTRAINT tasks_notes_len_check
    CHECK (notes IS NULL OR char_length(notes) <= 2000);

-- Dispatch boards read "today's work for this hub" on every load.
CREATE INDEX IF NOT EXISTS tasks_org_day_idx ON tasks (org_id, day);
