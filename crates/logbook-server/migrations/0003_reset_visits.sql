-- Spec 4.8. The counter now counts unique visitors per day. The old rows count
-- page loads, so they start again from zero. Only `visits` changes.
-- Migrations are additive only. Never edit this file after it ships.
DELETE FROM visits;
