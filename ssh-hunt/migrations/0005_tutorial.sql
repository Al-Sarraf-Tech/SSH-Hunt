-- Add tutorial progress column introduced by the story expansion feature.
-- Values: 0 = not started, 1-6 = current step, 7 = completed.
ALTER TABLE players ADD COLUMN IF NOT EXISTS tutorial_step SMALLINT NOT NULL DEFAULT 0;
